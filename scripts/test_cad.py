"""Run with the project's CadQuery environment: python scripts/test_cad.py."""
import importlib.util
from pathlib import Path
import tempfile
import unittest
import json
import struct
import math
import subprocess
import sys

path = Path(__file__).resolve().parents[1] / "apps/desktop/src-tauri/scripts/export_step.py"
spec = importlib.util.spec_from_file_location("cad_adapter", path)
adapter = importlib.util.module_from_spec(spec)
spec.loader.exec_module(adapter)
converter_spec = importlib.util.spec_from_file_location("step_converter", path.with_name("convert_step.py"))
converter = importlib.util.module_from_spec(converter_spec)
converter_spec.loader.exec_module(converter)

program_spec = importlib.util.spec_from_file_location("model_program", path.with_name("model_program.py"))
program = importlib.util.module_from_spec(program_spec)
program_spec.loader.exec_module(program)

class ProgramTests(unittest.TestCase):
    def test_feature_failure_has_identity_and_does_not_export(self):
        worker = path.with_name("model_program.py").read_text(encoding="utf-8")
        source = "feature_0 = cq.Workplane('XY').box(10,10,10)\nfeature_1 = feature_0.cut(feature_0)\nresult = feature_1"
        features = [{"featureId":"Pad","operation":"extrude","solid":True},
                    {"featureId":"Pocket","operation":"boolean","solid":True}]
        with tempfile.TemporaryDirectory() as tmp:
            completed = subprocess.run([sys.executable,"-I","-c",worker],
                input=json.dumps({"program":source,"features":features}).encode(),
                cwd=tmp,capture_output=True,timeout=30)
            self.assertNotEqual(completed.returncode, 0)
            report = json.loads(completed.stdout)
            self.assertEqual(report["featureId"], "Pocket")
            self.assertEqual(report["operation"], "boolean")
            self.assertEqual(report["code"], "GEOMETRY_BUILD_FAILED")
            self.assertFalse((Path(tmp)/"model.step").exists())

    def test_document_rejects_disconnected_body(self):
        engine = program.Interpreter()
        engine.feature_lines = {1:{"featureId":"Union","operation":"boolean","solid":True}}
        with self.assertRaisesRegex(ValueError, "disconnected"):
            engine.build("result = cq.Workplane('XY').box(1,1,1).union(cq.Workplane('XY').box(1,1,1).translate((5,0,0)))")

    def test_worker_reads_utf8_even_with_windows_text_stream(self):
        source = 'part = cq.Workplane("XY").box(2,2,2)\nparts = [["Шестерня",part,[0,0,1],[0,0,0],60]]'
        worker = path.with_name("model_program.py").read_text(encoding="utf-8")
        prefix = 'import io,sys\nsys.stdin = io.TextIOWrapper(sys.stdin.buffer, encoding="cp1251")\n'
        with tempfile.TemporaryDirectory() as tmp:
            subprocess.run([sys.executable, "-I", "-c", prefix + worker],
                input=json.dumps({"program":source},ensure_ascii=False).encode("utf-8"),
                cwd=tmp,check=True,capture_output=True,timeout=30)
            manifest=json.loads((Path(tmp)/"parts.json").read_text(encoding="utf-8"))
            self.assertEqual(manifest[0]["name"], "Шестерня")

    def test_result_composition_does_not_duplicate_motion_parts(self):
        engine = program.Interpreter()
        result = engine.build('''base = cq.Workplane("XY").box(40,40,2)
rotor = cq.Workplane("XY").circle(5).extrude(4).translate((0,0,5))
parts = [["base",base,[0,0,1],[0,0,0],0],["rotor",rotor,[0,0,1],[0,0,5],60]]
result = base.add(rotor)''')
        self.assertEqual(len(engine.parts[0][1].Solids()), 1)
        self.assertEqual(len(result.Solids()), 2)
        self.assertAlmostEqual(engine.parts[0][1].Volume(), 3200, places=5)
        self.assertAlmostEqual(result.Volume(), 3200 + math.pi*25*4, places=5)
        # The inverse ordering is also common in generated source.
        engine = program.Interpreter()
        result = engine.build('''base = cq.Workplane("XY").box(40,40,2)
rotor = cq.Workplane("XY").circle(5).extrude(4).translate((0,0,5))
result = base.add(rotor)
parts = [["base",base,[0,0,1],[0,0,0],0],["rotor",rotor,[0,0,1],[0,0,5],60]]''')
        self.assertEqual(len(engine.parts[0][1].Solids()), 1)
        self.assertEqual(len(result.Solids()), 2)

    def test_same_solid_cannot_belong_to_multiple_motion_components(self):
        with self.assertRaisesRegex(ValueError, "duplicated"):
            program.Interpreter().build('''rotor = cq.Workplane("XY").circle(5).extrude(4)
assembly = rotor.add(cq.Workplane("XY").box(40,40,2).translate((0,0,-5)))
parts = [["base",assembly,[0,0,1],[0,0,0],0],["rotor",rotor,[0,0,1],[0,0,0],60]]''')

    def test_cube_sphere_and_arbitrary_axis_holes(self):
        cube = program.Interpreter().build('result = cq.Workplane("XY").box(50,50,50)')
        self.assertAlmostEqual(cube.Volume(), 125000)
        sphere = program.Interpreter().build('result = cq.Workplane("XY").sphere(25)')
        self.assertAlmostEqual(sphere.Volume(), 4/3*math.pi*25**3, places=4)
        drilled = program.Interpreter().build('result = cq.Workplane("XY").sphere(25).cut(cq.Workplane("YZ").circle(10).extrude(60, both=True))')
        self.assertLess(drilled.Volume(), sphere.Volume())
        self.assertFalse(drilled.Solids()[0].isInside((0,0,0)))
        with tempfile.TemporaryDirectory() as tmp:
            step = Path(tmp)/"sphere.step"
            adapter.cq.exporters.export(drilled, str(step))
            metrics = converter.convert(step, Path(tmp)/"sphere.glb")
            self.assertEqual(metrics["solids"], 1)
            self.assertAlmostEqual(metrics["volumeMm3"], drilled.Volume(), places=3)

    def test_repeated_teeth_and_organic_composition(self):
        cog = program.Interpreter().build("""result = cq.Workplane("XY").circle(20).extrude(8)
for i in range(16):
    tooth = cq.Workplane("XY").box(7,3,8).translate((22,0,4)).rotate((0,0,0),(0,0,1),i*360/16)
    result = result.union(tooth)
result = result.faces(">Z").workplane().hole(8)
""")
        self.assertEqual(len(cog.Solids()), 1)
        self.assertGreater(cog.Volume(), math.pi*20**2*8)
        animal = program.Interpreter().build("""result = cq.Workplane("XY").sphere(15)
head = cq.Workplane("XY").sphere(10).translate((17,0,9))
result = result.union(head)
for x in [-8,8]:
    for y in [-7,7]:
        leg = cq.Workplane("XY").circle(3).extrude(17).translate((x,y,-23))
        result = result.union(leg)
""")
        self.assertEqual(len(animal.Solids()), 1)

    def test_motion_parts_keep_separate_nodes_and_pivots(self):
        engine=program.Interpreter()
        shape=engine.build('a = cq.Workplane("XY").box(10,4,2).translate((20,0,2))\nb = cq.Workplane("XY").box(40,20,1)\nparts = [["gear",a,[0,0,1],[20,0,2],36],["base",b,[0,0,1],[0,0,0],0]]')
        self.assertEqual(len(engine.parts),2)
        with tempfile.TemporaryDirectory() as tmp:
            source=Path(tmp)/"assembly.step"
            preview=Path(tmp)/"assembly.glb"
            adapter.cq.exporters.export(shape,str(source))
            components=[dict(name=name,shape=solid,axis=axis,pivot=pivot,speed=speed) for name,solid,axis,pivot,speed in engine.parts]
            converter.convert(source,preview,components)
            raw=preview.read_bytes(); length=struct.unpack_from('<I',raw,12)[0]; doc=json.loads(raw[20:20+length])
            self.assertEqual(len(doc['nodes']),2)
            self.assertEqual(doc['nodes'][0]['translation'],[.02,.002,0])
            self.assertEqual(doc['nodes'][0]['extras']['formaMotion'],{'axis':[0,1,0],'speed':36})
            self.assertEqual(doc['nodes'][1]['extras']['formaMotion']['speed'],0)
        with self.assertRaises(ValueError):
            program.Interpreter().build('a = cq.Workplane("XY").box(1,1,1)\nparts = [["bad",a,[0,0,0],[0,0,0],20]]')

    def test_imported_base_edits(self):
        base = adapter.cq.Workplane("XY").box(50,50,50)
        result = program.Interpreter(base).build('result = base.faces(">Z").workplane().hole(20)')
        self.assertAlmostEqual(result.Volume(), 125000-math.pi*10**2*50, places=3)

    def test_non_geometry_code_and_unbounded_work_rejected(self):
        for source in ['import os', 'result = open("secret")', 'result = cq.Workplane.__class__',
                       'result = cq.Workplane("XY").__getattribute__("__class__")',
                       'for i in range(1001):\n    x = i', 'while True:\n    x = 1',
                       'result = cq.Workplane("XY")']:
            with self.subTest(source=source), self.assertRaises((ValueError, TypeError)):
                program.Interpreter().build(source)


class CadTests(unittest.TestCase):
    def test_cube_center_hole_has_real_removed_volume(self):
        p=dict(kind="box",width=50,depth=50,height=50,thickness=5,holeDiameter=20,holes=1)
        shape=adapter.build(p).val()
        self.assertTrue(shape.isValid())
        self.assertAlmostEqual(shape.Volume(),50**3-math.pi*10**2*50,places=4)
        self.assertFalse(shape.isInside((0,0,25)))
        self.assertTrue(shape.isInside((15,0,25)))
        with tempfile.TemporaryDirectory() as tmp:
            path=str(Path(tmp)/"hole.step")
            adapter.cq.exporters.export(shape,path)
            loaded=adapter.cq.importers.importStep(path).val()
            self.assertAlmostEqual(loaded.Volume(),shape.Volume(),places=4)

    def test_four_cube_holes_remove_material(self):
        shape=adapter.build(dict(kind="box",width=50,depth=50,height=50,thickness=5,holeDiameter=12.5,holes=4)).val()
        self.assertTrue(shape.isValid())
        self.assertAlmostEqual(shape.Volume(),50**3-4*math.pi*6.25**2*50,places=4)
        for x in (-16,16):
            for y in (-14,14):
                self.assertFalse(shape.isInside((x,y,25)))

    def test_step_preview_units_and_indices(self):
        with tempfile.TemporaryDirectory() as tmp:
            source, output = Path(tmp)/"part.step", Path(tmp)/"preview.glb"
            adapter.cq.exporters.export(adapter.cq.Workplane("XY").box(120, 65, 60), str(source))
            metrics = converter.convert(source, output)
            self.assertEqual(metrics["solids"], 1)
            raw = output.read_bytes()
            magic, version, length = struct.unpack_from("<III", raw)
            self.assertEqual((magic, version, length), (0x46546C67, 2, len(raw)))
            json_length = struct.unpack_from("<I", raw, 12)[0]
            gltf = json.loads(raw[20:20+json_length])
            pos = gltf["accessors"][0]
            for extent, expected in zip([b-a for a,b in zip(pos["min"],pos["max"])], [0.12,0.06,0.065]):
                self.assertAlmostEqual(extent, expected)
            binary_start = 28+json_length
            index_start = binary_start+gltf["bufferViews"][1]["byteOffset"]
            indices = struct.unpack_from("<"+"I"*gltf["accessors"][1]["count"],raw,index_start)
            self.assertLess(max(indices),pos["count"])

    def test_templates_round_trip(self):
        for kind in ("box", "plate", "bracket", "enclosure", "cylinder"):
            with self.subTest(kind=kind), tempfile.TemporaryDirectory() as tmp:
                p = dict(kind=kind, width=120, depth=65, height=60, thickness=5, holeDiameter=8, holes=4)
                model = adapter.build(p)
                self.assertTrue(model.val().isValid())
                self.assertGreater(model.val().Volume(), 0)
                self.assertEqual(len(model.solids().vals()), 1)
                out = str(Path(tmp) / "model.step")
                adapter.cq.exporters.export(model, out)
                read = adapter.cq.importers.importStep(out)
                self.assertTrue(read.val().isValid())
                self.assertAlmostEqual(read.val().Volume(), model.val().Volume(), places=3)

    def test_bad_dimensions_rejected(self):
        p = dict(kind="plate", width=120, depth=65, height=60, thickness=80, holeDiameter=8, holes=4)
        with self.assertRaises(AssertionError):
            adapter.build(p)


if __name__ == "__main__":
    unittest.main()
