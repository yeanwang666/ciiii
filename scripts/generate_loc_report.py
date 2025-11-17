#!/usr/bin/env python3
import json
import subprocess
from pathlib import Path
import argparse

parser = argparse.ArgumentParser()
parser.add_argument("--workspace", required=True)
parser.add_argument("--clone-dir", required=True)
parser.add_argument("--output", required=True)
args = parser.parse_args()

workspace = Path(args.workspace)
clone_dir = Path(args.clone_dir)
output_dir = Path(args.output)

targets = [workspace] + list(clone_dir.iterdir())
results = {}

def tokei(path: Path):
    try:
        out = subprocess.check_output(
            ["tokei", path.as_posix(), "--output", "json"],
            stderr=subprocess.DEVNULL,
        )
        return json.loads(out)
    except Exception as e:
        return {"error": str(e)}

for t in targets:
    print(f"[tokei] scanning {t}")
    results[t.name] = tokei(t)

with open(output_dir / "loc.json", "w") as f:
    json.dump(results, f, indent=2)

# Markdown report
with open(output_dir / "loc.md", "w") as f:
    f.write("# StarryOS 代码统计\n\n")
    for name, data in results.items():
        f.write(f"## {name}\n\n")
        if "error" in data:
            f.write(f"⚠ Error: {data['error']}\n\n")
            continue
        total = sum(v.get("code", 0) for v in data.values())
        f.write(f"总代码行数：**{total} 行**\n\n")
        f.write("语言分布：\n")
        f.write("```\n")
        f.write(json.dumps(data, indent=2))
        f.write("\n```\n")

print("[report] Generated loc.json and loc.md")
