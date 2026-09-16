"""Geometry-only AST interpreter. Agent source is never passed to exec/eval.

No Python object introspection, files, imports, callbacks or arbitrary functions.
Only explicit CAD operations and bounded arithmetic/control flow are exposed.
"""
import ast
import json
import math
import operator
import sys
import cadquery as cq

METHODS = set("box sphere cylinder cone circle ellipse rect polygon polyline spline lineTo moveTo move line hLine vLine hLineTo vLineTo threePointArc radiusArc tangentArcPoint close wire toPending extrude twistExtrude revolve loft sweep union cut intersect hole cboreHole cskHole cutThruAll cutBlind shell fillet chamfer faces edges vertices solids workplane transformed center pushPoints polarArray rarray translate rotate mirror mirrorX mirrorY add val vals clean combine split section offset2D slot2D text".split())
SHAPE_METHODS = set("translate rotate mirror scale fuse cut intersect fillet chamfer clean Solids Faces Edges Vertices isValid Volume Area BoundingBox Center".split())
CONSTRUCTORS = {"Workplane": cq.Workplane, "Vector": cq.Vector, "Plane": cq.Plane}
MATH = {name: getattr(math, name) for name in ("sin", "cos", "tan", "sqrt", "atan2", "acos", "asin", "radians", "degrees", "floor", "ceil", "pi", "e")}
OPS = {ast.Add: operator.add, ast.Sub: operator.sub, ast.Mult: operator.mul,
       ast.Div: operator.truediv, ast.FloorDiv: operator.floordiv, ast.Mod: operator.mod, ast.Pow: operator.pow}
CMP = {ast.Lt: operator.lt, ast.LtE: operator.le, ast.Gt: operator.gt,
       ast.GtE: operator.ge, ast.Eq: operator.eq, ast.NotEq: operator.ne}


class Interpreter:
    def __init__(self, base=None):
        self.env = {"base": base} if base is not None else {}
        self.steps = 0
        self.parts = []

    def tick(self):
        self.steps += 1
        if self.steps > 50000:
            raise ValueError("Program exceeds 50,000 evaluation steps")

    def value(self, node):
        self.tick()
        if isinstance(node, ast.Constant) and isinstance(node.value, (int, float, str, bool, type(None))):
            return node.value
        if isinstance(node, ast.Name) and node.id in self.env:
            return self.env[node.id]
        if isinstance(node, (ast.List, ast.Tuple)):
            return [self.value(v) for v in node.elts]
        if isinstance(node, ast.BinOp) and type(node.op) in OPS:
            a, b = self.value(node.left), self.value(node.right)
            if not isinstance(a, (int, float)) or not isinstance(b, (int, float)):
                raise ValueError("Arithmetic requires numbers")
            if isinstance(node.op, ast.Pow) and abs(b) > 20:
                raise ValueError("Exponent exceeds limit")
            result = OPS[type(node.op)](a, b)
            if not isinstance(result, (int, float)) or not math.isfinite(result) or abs(result) > 1e12:
                raise ValueError("Arithmetic exceeds limit")
            return result
        if isinstance(node, ast.UnaryOp) and isinstance(node.op, (ast.USub, ast.UAdd, ast.Not)):
            v = self.value(node.operand)
            if isinstance(node.op, ast.Not):
                return not v
            if not isinstance(v, (int, float)):
                raise ValueError("Unary arithmetic requires a number")
            return -v if isinstance(node.op, ast.USub) else v
        if isinstance(node, ast.Compare):
            values = [self.value(node.left)] + [self.value(v) for v in node.comparators]
            return all(CMP[type(op)](values[i], values[i+1]) for i, op in enumerate(node.ops))
        if isinstance(node, ast.IfExp):
            return self.value(node.body if self.value(node.test) else node.orelse)
        if isinstance(node, ast.Subscript):
            seq, index = self.value(node.value), self.value(node.slice)
            if type(seq) not in (list, tuple) or type(index) is not int:
                raise ValueError("Only list indexing is supported")
            return seq[index]
        if isinstance(node, ast.Attribute) and isinstance(node.value, ast.Name) and node.value.id == "math" and node.attr in ("pi", "e"):
            return MATH[node.attr]
        if isinstance(node, ast.Call):
            args = [self.value(v) for v in node.args]
            if any(k.arg is None or k.arg.startswith("_") for k in node.keywords):
                raise ValueError("Keyword expansion is unavailable")
            kwargs = {k.arg: self.value(k.value) for k in node.keywords}
            fn = node.func
            target = None
            if isinstance(fn, ast.Name):
                target = {"range": self.bounded_range, "abs": abs, "min": min, "max": max, "round": round, "int": int, "float": float, "len": len}.get(fn.id)
            elif isinstance(fn, ast.Attribute):
                if isinstance(fn.value, ast.Name) and fn.value.id in ("cq", "math"):
                    target = (CONSTRUCTORS if fn.value.id == "cq" else MATH).get(fn.attr)
                else:
                    obj = self.value(fn.value)
                    allowed = METHODS if isinstance(obj, cq.Workplane) else SHAPE_METHODS if isinstance(obj, cq.Shape) else {"append"} if type(obj) is list else set()
                    if fn.attr in allowed:
                        if type(obj) is list and len(obj) >= 10000:
                            raise ValueError("List exceeds limit")
                        # Workplane.add mutates its receiver, unlike most CQ operations.
                        # Keep expression composition from mutating a component already
                        # referenced by parts (e.g. result = base.add(rotor)).
                        if isinstance(obj, cq.Workplane) and fn.attr == "add":
                            obj = obj.newObject(list(obj.objects))
                        target = getattr(obj, fn.attr)
            if not callable(target):
                raise ValueError("This operation is not in the geometry API")
            return target(*args, **kwargs)
        raise ValueError(f"Unsupported expression: {type(node).__name__}")

    @staticmethod
    def bounded_range(*args):
        result = range(*args)
        if len(result) > 1000:
            raise ValueError("Loop exceeds 1,000 iterations")
        return result

    def assign(self, target, value):
        if isinstance(target, ast.Name) and not target.id.startswith("_") and target.id not in ("cq", "math"):
            self.env[target.id] = value
        elif isinstance(target, (ast.Tuple, ast.List)) and type(value) in (list, tuple) and len(target.elts) == len(value):
            for t, v in zip(target.elts, value):
                self.assign(t, v)
        else:
            raise ValueError("Only local variable assignment is available")

    def block(self, statements):
        for node in statements:
            self.tick()
            feature = getattr(self, "feature_lines", {}).get(node.lineno)
            if feature:
                self.active_feature = feature
            if isinstance(node, ast.Assign):
                value = self.value(node.value)
                if feature and feature["solid"]:
                    shapes = value.vals() if isinstance(value, cq.Workplane) else [value]
                    solids = [s for shape in shapes if isinstance(shape, cq.Shape) for s in shape.Solids()]
                    if not solids or any(not s.isValid() or s.Volume() <= 0 for s in solids):
                        raise ValueError("Feature must produce valid, nonempty solids")
                    if len(solids) != 1:
                        raise ValueError("Feature produced disconnected solids; this document version requires one solid per body")
                for target in node.targets:
                    self.assign(target, value)
            elif isinstance(node, ast.Expr):
                self.value(node.value)
            elif isinstance(node, ast.For):
                seq = self.value(node.iter)
                if type(seq) not in (list, tuple, range) or len(seq) > 1000:
                    raise ValueError("Invalid loop")
                for value in seq:
                    self.assign(node.target, value)
                    self.block(node.body)
                self.block(node.orelse)
            elif isinstance(node, ast.If):
                self.block(node.body if self.value(node.test) else node.orelse)
            else:
                raise ValueError(f"Unsupported statement: {type(node).__name__}")

    def build(self, source):
        if len(source) > 60000:
            raise ValueError("Program exceeds 60,000 characters")
        self.block(ast.parse(source).body)
        parts = self.env.get("parts")
        if parts is not None:
            if type(parts) is not list or not 1 <= len(parts) <= 128:
                raise ValueError("parts must contain 1..128 named components")
            names = set()
            owners = []
            for entry in parts:
                if type(entry) is not list or len(entry) != 5:
                    raise ValueError("Each part is [name, shape, axis, pivot, degrees_per_second]")
                name, shape, axis, pivot, speed = entry
                if not isinstance(name, str) or not name or len(name) > 80 or name in names:
                    raise ValueError("Part names must be unique and at most 80 characters")
                names.add(name)
                if isinstance(shape, cq.Workplane):
                    shape = cq.Compound.makeCompound([v for v in shape.vals() if isinstance(v, cq.Shape)])
                if not isinstance(shape, cq.Shape) or not shape.isValid() or not shape.Solids():
                    raise ValueError("Each component must contain valid solids")
                solids = shape.Solids()
                for solid in solids:
                    for previous, owner in owners:
                        if solid.isSame(previous):
                            raise ValueError(f"Solid is duplicated in parts '{owner}' and '{name}'. Keep each solid in only one motion component.")
                    owners.append((solid, name))
                for vector in [axis, pivot]:
                    if type(vector) is not list or len(vector) != 3 or not all(type(v) in (int,float) and math.isfinite(v) and abs(v) <= 100000 for v in vector):
                        raise ValueError("Motion axis and pivot must be finite XYZ vectors")
                if not isinstance(speed, (float,int)) or not math.isfinite(speed) or abs(speed) > 7200 or sum(v*v for v in axis) < 1e-12:
                    raise ValueError("Invalid rotation speed or axis")
                self.parts.append((name,shape,axis,pivot,speed))
            self.env["result"] = cq.Compound.makeCompound([part[1] for part in self.parts])
        result = self.env.get("result")
        if isinstance(result, cq.Workplane):
            shapes = [v for v in result.vals() if isinstance(v, cq.Shape)]
            result = cq.Compound.makeCompound(shapes)
        if not isinstance(result, cq.Shape) or not result.isValid() or not result.Solids() or result.Volume() <= 0:
            raise ValueError("result must contain valid, nonempty solids")
        bounds = result.BoundingBox()
        if max(bounds.xlen, bounds.ylen, bounds.zlen) > 100000 or len(result.Solids()) > 1000:
            raise ValueError("Model exceeds size or solid count limit")
        return result


if __name__ == "__main__":
    try:
        request = json.load(sys.stdin.buffer)
        base = cq.importers.importStep("base.step") if request.get("hasBase") else None
        interpreter = Interpreter(base)
        interpreter.feature_lines = {i + 1: feature for i, feature in enumerate(request.get("features") or [])}
        result = interpreter.build(request["program"])
        cq.exporters.export(result, "model.step")
        manifest = []
        for index, (name, shape, axis, pivot, speed) in enumerate(interpreter.parts):
            filename = f"part-{index}.step"
            cq.exporters.export(shape, filename)
            manifest.append({"name":name,"source":filename,"axis":axis,"pivot":pivot,"speed":speed})
        from pathlib import Path
        Path("parts.json").write_text(json.dumps(manifest), encoding="utf-8")

        print(json.dumps({"ok": True, "volume": result.Volume(), "solidCount": len(result.Solids()), "usesBase": any(isinstance(node, ast.Name) and node.id == "base" for node in ast.walk(ast.parse(request["program"])))}))
    except Exception as error:
        feature = getattr(locals().get("interpreter"), "active_feature", {})
        print(json.dumps({"type": "error", "code": "GEOMETRY_BUILD_FAILED", "featureId": feature.get("featureId"), "operation": feature.get("operation"), "message": "CAD: " + str(error)[:1500]}))
        sys.exit(1)
