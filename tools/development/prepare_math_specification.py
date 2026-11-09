# SPDX-License-Identifier: AGPL-3.0-only
"""Keep literal specification sources and report explicitly contextualized test inputs.

Optional test_script/test_notes fields add review context without replacing the
source inventory's hancom_script/source_text. input_script is the expression
actually sent to the converter and native renderer for every reported case.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]


def prepare(fixture, converter, output):
    data = json.loads(fixture.read_text(encoding="utf-8"))
    source_pdf = ROOT / data["source"]["path"]
    if hashlib.sha256(source_pdf.read_bytes()).hexdigest() != data["source"]["sha256"]:
        raise ValueError("Specification PDF changed; review the extraction first.")
    cases, native, scripts, ids, inputs = [], [], {}, set(), set()

    def native_id(script):
        if script not in scripts:
            ident = f"native-{len(scripts) + 1:04}"
            scripts[script] = ident
            native.append({"case_id": ident, "name": ident, "script": script})
        return scripts[script]

    def convert(direction, value):
        result = subprocess.run([str(converter), direction], input=value,
                                encoding="utf-8", capture_output=True, check=False)
        if result.returncode:
            return {"ok": False, "error": result.stderr.rstrip("\r\n"),
                    "exit_code": result.returncode}
        return {"ok": True, "output": result.stdout.rstrip("\r\n")}

    for original in data["cases"]:
        if original["id"] in ids:
            raise ValueError("Duplicate case ID")
        ids.add(original["id"])
        item = dict(original)
        item["input_script"] = item.get("test_script", item["hancom_script"])
        if not isinstance(item["input_script"], str):
            raise ValueError(f"Input script must be text: {item['id']}")
        if item["input_script"] in inputs:
            raise ValueError(f"Duplicate test input; combine source records: {item['id']}")
        inputs.add(item["input_script"])
        item["native_original_id"] = native_id(item["input_script"])
        item["hml_to_tex"] = convert("hml-to-tex", item["input_script"])
        item["tex_to_hml"] = {"ok": False, "skipped": True,
                              "error": "HML→TeX 변환 결과가 없어 실행하지 않았습니다."}
        if item["hml_to_tex"]["ok"]:
            item["tex_to_hml"] = convert("tex-to-hml", item["hml_to_tex"]["output"])
            if item["tex_to_hml"]["ok"]:
                item["native_roundtrip_id"] = native_id(item["tex_to_hml"]["output"])
        cases.append(item)
    output.mkdir(parents=True, exist_ok=True)
    summary = {"count": len(cases), "unique_native_scripts": len(native),
               "hml_to_tex_ok": sum(c["hml_to_tex"]["ok"] for c in cases),
               "tex_to_hml_ok": sum(c.get("tex_to_hml", {}).get("ok", False) for c in cases)}
    report = {"title": data["title"], "source": data["source"], "notice": data["notice"],
              "exclusions": data.get("exclusions", []), "summary": summary, "cases": cases}
    (output / "report.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    (output / "native-input.json").write_text(json.dumps({"cases": native}, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(summary), flush=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fixture", type=Path, default=ROOT / "crates/md2hwp-math/tests/fixtures/specification-equations.json")
    parser.add_argument("--converter", type=Path, default=ROOT / "target/debug/md2hwp-math.exe")
    parser.add_argument("--output", type=Path, default=ROOT / "artifacts/math-spec-examples")
    args = parser.parse_args()
    prepare(args.fixture.resolve(), args.converter.resolve(), args.output.resolve())
