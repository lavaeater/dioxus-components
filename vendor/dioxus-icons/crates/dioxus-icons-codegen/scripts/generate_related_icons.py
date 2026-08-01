#!/usr/bin/env python3
"""Generate same-set related icon data with SigLIP2 image/text score fusion."""

from __future__ import annotations

import argparse
import io
import json
import os
import shutil
import subprocess
from pathlib import Path

os.environ.setdefault("HF_HUB_DISABLE_PROGRESS_BARS", "1")

import torch
import torch.nn.functional as F
from PIL import Image
from transformers import AutoModel, AutoProcessor
from transformers.utils import logging


DEFAULT_MODEL = "google/siglip2-base-patch16-224"
DEFAULT_RELATED_COUNT = 6
DEFAULT_RENDER_SIZE = 224
DEFAULT_TEXT_WEIGHT = 0.35


def main() -> None:
    args = parse_args()
    if not shutil.which("rsvg-convert"):
        raise SystemExit("rsvg-convert is required to render SVG icons")

    svg_paths = sorted(args.icons_dir.glob("*.svg"))
    if not svg_paths:
        raise SystemExit(f"no SVG files found in {args.icons_dir}")
    if len(svg_paths) <= args.related_count:
        raise SystemExit("related count must be lower than the number of icons")
    if not 0 <= args.text_weight <= 1:
        raise SystemExit("text weight must be between 0 and 1")

    names = [path.stem for path in svg_paths]
    device = select_device(args.device)

    metadata_dir = args.metadata_dir or args.icons_dir
    descriptions = [icon_description(path, metadata_dir) for path in svg_paths]

    print(f"loading {args.model} on {device}", flush=True)
    logging.disable_progress_bar()
    processor = AutoProcessor.from_pretrained(args.model)
    model = AutoModel.from_pretrained(args.model).to(device).eval()

    print(f"rendering and embedding {len(svg_paths)} icons", flush=True)
    image_embeddings = []
    for start in range(0, len(svg_paths), args.batch_size):
        batch_paths = svg_paths[start : start + args.batch_size]
        images = [render_svg(path, args.render_size) for path in batch_paths]
        inputs = processor(images=images, return_tensors="pt")
        inputs = {key: value.to(device) for key, value in inputs.items()}

        with torch.no_grad():
            features = model.get_image_features(**inputs)

        features = pooled_features(features)

        image_embeddings.append(F.normalize(features.float().cpu(), dim=-1))
        print(
            f"embedded images {min(start + args.batch_size, len(svg_paths))}/{len(svg_paths)}",
            flush=True,
        )

    text_embeddings = []
    if args.text_weight > 0:
        print(f"embedding {len(descriptions)} icon descriptions", flush=True)
        for start in range(0, len(descriptions), args.batch_size):
            batch_descriptions = descriptions[start : start + args.batch_size]
            inputs = processor(
                text=batch_descriptions,
                return_tensors="pt",
                padding=True,
                truncation=True,
            )
            inputs = {key: value.to(device) for key, value in inputs.items()}

            with torch.no_grad():
                features = model.get_text_features(**inputs)

            features = pooled_features(features)
            text_embeddings.append(F.normalize(features.float().cpu(), dim=-1))
            print(
                f"embedded text {min(start + args.batch_size, len(descriptions))}/{len(descriptions)}",
                flush=True,
            )

    image_matrix = torch.cat(image_embeddings, dim=0)
    image_similarity = image_matrix @ image_matrix.T
    if text_embeddings:
        text_matrix = torch.cat(text_embeddings, dim=0)
        text_similarity = text_matrix @ text_matrix.T
        similarity = (1.0 - args.text_weight) * image_similarity
        similarity += args.text_weight * text_similarity
        embedding_kind = "image_text_score_fusion"
    else:
        similarity = image_similarity
        embedding_kind = "image"

    items = []
    for index, name in enumerate(names):
        scores = similarity[index].tolist()
        ranked = sorted(
            (
                (score, candidate)
                for candidate_index, (score, candidate) in enumerate(zip(scores, names))
                if candidate_index != index
            ),
            key=lambda entry: (-entry[0], entry[1]),
        )
        items.append(
            {
                "name": name,
                "related": [candidate for _, candidate in ranked[: args.related_count]],
            }
        )

    output = {
        "model": args.model,
        "model_collection": "https://huggingface.co/collections/google/siglip2",
        "embedding": embedding_kind,
        "icon_set": args.icon_set,
        "icon_set_version": args.icon_set_version,
        "related_count": args.related_count,
        "render_size": args.render_size,
        "image_weight": 1.0 - args.text_weight,
        "text_weight": args.text_weight,
        "items": items,
    }

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(output, indent=2) + "\n")
    print(f"wrote {args.output}", flush=True)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--icons-dir", type=Path, required=True)
    parser.add_argument("--metadata-dir", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--icon-set", default="lucide")
    parser.add_argument("--icon-set-version", required=True)
    parser.add_argument("--model", default=DEFAULT_MODEL)
    parser.add_argument("--related-count", type=int, default=DEFAULT_RELATED_COUNT)
    parser.add_argument("--render-size", type=int, default=DEFAULT_RENDER_SIZE)
    parser.add_argument("--text-weight", type=float, default=DEFAULT_TEXT_WEIGHT)
    parser.add_argument("--batch-size", type=int, default=32)
    parser.add_argument("--device", default="auto", choices=["auto", "cpu", "cuda", "mps"])
    return parser.parse_args()


def select_device(requested: str) -> str:
    if requested != "auto":
        return requested
    if torch.cuda.is_available():
        return "cuda"
    if torch.backends.mps.is_available():
        return "mps"
    return "cpu"


def render_svg(path: Path, size: int) -> Image.Image:
    result = subprocess.run(
        ["rsvg-convert", "--width", str(size), "--height", str(size), str(path)],
        check=True,
        stdout=subprocess.PIPE,
    )
    image = Image.open(io.BytesIO(result.stdout)).convert("RGBA")
    background = Image.new("RGBA", image.size, "white")
    background.alpha_composite(image)
    return background.convert("RGB")


def icon_description(svg_path: Path, metadata_dir: Path) -> str:
    metadata_path = metadata_dir / f"{svg_path.stem}.json"
    metadata = json.loads(metadata_path.read_text())
    label = svg_path.stem.replace("-", " ")
    tags = ", ".join(metadata.get("tags", []))
    categories = ", ".join(metadata.get("categories", []))
    return f"{label} icon. Tags: {tags}. Categories: {categories}."


def pooled_features(features: object) -> torch.Tensor:
    if hasattr(features, "pooler_output"):
        return features.pooler_output
    if isinstance(features, tuple):
        return features[0]
    return features


if __name__ == "__main__":
    main()
