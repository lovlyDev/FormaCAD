"""Exercise a self-contained Unix CAD worker outside the build directory."""

import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile


def main() -> None:
    if len(sys.argv) != 2:
        raise SystemExit("Usage: check-native-unix.py WORKER")
    worker = Path(sys.argv[1]).resolve(strict=True)
    with tempfile.TemporaryDirectory(prefix="forma-native-smoke-") as temporary:
        root = Path(temporary)
        isolated_worker = root / f"forma-cad-worker{worker.suffix}"
        shutil.copy2(worker, isolated_worker)
        request = {
            "protocolVersion": 1,
            "requestId": "packaged_worker_smoke",
            "bodyId": "body",
            "document": {
                "schemaVersion": 2,
                "revisionId": "revision_smoke",
                "parameters": [],
                "features": [
                    {
                        "id": "profile",
                        "name": "Profile",
                        "operation": {
                            "type": "rectangle",
                            "width": {"kind": "literal", "mm": 10},
                            "depth": {"kind": "literal", "mm": 20},
                        },
                    },
                    {
                        "id": "pad",
                        "name": "Pad",
                        "operation": {
                            "type": "extrude",
                            "sketchId": "profile",
                            "distance": {"kind": "literal", "mm": 5},
                        },
                    },
                ],
                "bodies": [
                    {"id": "body", "name": "Plate", "sourceFeatureId": "pad"}
                ],
            },
        }
        result = subprocess.run(
            [str(isolated_worker)],
            input=json.dumps(request),
            text=True,
            cwd=root,
            capture_output=True,
            timeout=30,
            check=True,
        )
        response = json.loads((root / "result.json").read_text(encoding="utf-8"))
        if response["status"] != "completed" or response["requestId"] != request["requestId"]:
            raise RuntimeError(f"CAD worker response is invalid: {response}")
        if abs(response["volumeMm3"] - 1000.0) > 0.001:
            raise RuntimeError("CAD worker produced the wrong volume")
        for name, key in (("model.step", "stepSha256"), ("preview.glb", "previewSha256")):
            digest = hashlib.sha256((root / name).read_bytes()).hexdigest()
            if digest != response[key]:
                raise RuntimeError(f"CAD worker checksum mismatch: {name}")
        if (root / "preview.glb").read_bytes()[:4] != b"glTF":
            raise RuntimeError("CAD worker did not produce a GLB preview")
        if result.stderr:
            print(result.stderr, file=sys.stderr)
    print("Isolated native CAD worker passed")


if __name__ == "__main__":
    main()
