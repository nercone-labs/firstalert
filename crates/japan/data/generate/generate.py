"""Builds the data of the `japan` crate from its sources.

Run with the sources downloaded into a directory of their own (never run from inside it):

    python3 -I generate.py SOURCES OUTPUT

SOURCES holds:

- ``admin1.geojson``: Natural Earth 1:10m Admin 1 – States, Provinces
  (https://raw.githubusercontent.com/nvkelso/natural-earth-vector/master/geojson/ne_10m_admin_1_states_provinces.geojson),
  public domain.
- ``shindo-name.html``: JMA, 緊急地震速報や震度情報で用いる区域の名称
  (https://www.data.jma.go.jp/svd/eqev/data/joho/shindo-name.html).
- ``t-yohokuinfo.html``: JMA, 津波予報区
  (https://www.data.jma.go.jp/svd/eqev/data/joho/t-yohokuinfo.html).

OUTPUT receives ``prefectures.bin`` (the format is documented in ``src/boundaries.rs``),
``areas.tsv`` and ``tsunami_areas.tsv``.
"""

import json
import struct
import sys
from html.parser import HTMLParser
from pathlib import Path
from typing import Dict, List, Optional, Tuple

Point = Tuple[float, float]

# Douglas–Peucker tolerance in degrees (about 500 m), and the smallest island kept, in square
# degrees (about 5 km²): enough for a map of the country or of a region on a phone.
TOLERANCE: float = 0.005
MIN_AREA: float = 0.0005


class Simplifier:
    """Simplification of rings of points."""

    @staticmethod
    def douglas_peucker(points: List[Point], tolerance: float) -> List[Point]:
        if len(points) < 3:
            return points
        keep: List[bool] = [False] * len(points)
        keep[0] = keep[-1] = True
        stack: List[Tuple[int, int]] = [(0, len(points) - 1)]
        while stack:
            a, b = stack.pop()
            ax, ay = points[a]
            bx, by = points[b]
            dx, dy = bx - ax, by - ay
            length: float = (dx * dx + dy * dy) ** 0.5
            best, index = -1.0, -1
            for i in range(a + 1, b):
                px, py = points[i]
                distance = abs(dy * (px - ax) - dx * (py - ay)) / length if length > 0 else ((px - ax) ** 2 + (py - ay) ** 2) ** 0.5
                if distance > best:
                    best, index = distance, i
            if best > tolerance:
                keep[index] = True
                stack += [(a, index), (index, b)]
        return [p for p, k in zip(points, keep) if k]

    @staticmethod
    def area(ring: List[Point]) -> float:
        return abs(sum(ring[i][0] * ring[i + 1][1] - ring[i + 1][0] * ring[i][1] for i in range(len(ring) - 1))) / 2


class Boundaries:
    """The outlines of the prefectures, as ``prefectures.bin``."""

    @staticmethod
    def build(geojson: Path) -> bytes:
        data = json.loads(geojson.read_text(encoding="utf-8"))
        features = [f for f in data["features"] if f["properties"].get("iso_a2") == "JP"]
        features.sort(key=lambda f: int(f["properties"]["iso_3166_2"][3:]))
        out = bytearray(b"FAJP") + bytes([1, len(features)])
        for feature in features:
            geometry = feature["geometry"]
            polygons = geometry["coordinates"] if geometry["type"] == "MultiPolygon" else [geometry["coordinates"]]
            rings: List[List[Point]] = []
            for polygon in polygons:
                outer: List[Point] = [tuple(p) for p in polygon[0]]
                if Simplifier.area(outer) < MIN_AREA:
                    continue
                simplified = Simplifier.douglas_peucker(outer, TOLERANCE)
                if simplified[0] != simplified[-1]:
                    simplified.append(simplified[0])
                if len(simplified) >= 4:
                    rings.append(simplified[:-1])
            if not rings:
                largest = max((list(map(tuple, p[0])) for p in polygons), key=Simplifier.area)
                rings = [Simplifier.douglas_peucker(largest, TOLERANCE)[:-1]]
            out += bytes([int(feature["properties"]["iso_3166_2"][3:])]) + struct.pack("<H", len(rings))
            for ring in rings:
                out += struct.pack("<H", len(ring))
                for x, y in ring:
                    out += struct.pack("<ii", round(x * 1e4), round(y * 1e4))
        return bytes(out)


class Table(HTMLParser):
    """The rows of the tables of an HTML page, each a list of (text, rowspan)."""

    def __init__(self) -> None:
        super().__init__()
        self.rows: List[List[Tuple[str, int]]] = []
        self.row: Optional[List[Tuple[str, int]]] = None
        self.cell: Optional[List] = None

    def handle_starttag(self, tag: str, attrs: List[Tuple[str, Optional[str]]]) -> None:
        if tag == "tr":
            self.row = []
        elif tag in ("td", "th") and self.row is not None:
            self.cell = ["", int(dict(attrs).get("rowspan") or "1")]

    def handle_endtag(self, tag: str) -> None:
        if tag in ("td", "th") and self.cell is not None and self.row is not None:
            self.row.append((self.cell[0].strip(), self.cell[1]))
            self.cell = None
        elif tag == "tr" and self.row is not None:
            self.rows.append(self.row)
            self.row = None

    def handle_data(self, data: str) -> None:
        if self.cell is not None:
            self.cell[0] += data


class Areas:
    """The areas of the warnings and of tsunami forecasts, as tab-separated lists."""

    @staticmethod
    def seismic(page: Path) -> List[Tuple[str, str, str]]:
        table = Table()
        table.feed(page.read_text(encoding="utf-8"))
        pending: List[Optional[Tuple[str, int]]] = [None] * 4
        found: List[Tuple[str, str, str]] = []
        for row in table.rows:
            cells = list(row)
            full: List[Optional[str]] = []
            for i in range(4):
                carried = pending[i]
                if carried and carried[1] > 0:
                    full.append(carried[0])
                    pending[i] = (carried[0], carried[1] - 1)
                elif cells:
                    text, span = cells.pop(0)
                    full.append(text)
                    pending[i] = (text, span - 1)
                else:
                    full.append(None)
            if None in full or full[0] == "都道府県名":
                continue
            key = (full[0], full[1], full[2])
            if key not in found:
                found.append(key)  # type: ignore[arg-type]
        return found

    @staticmethod
    def tsunami(page: Path, prefectures: List[str]) -> List[Tuple[str, List[str]]]:
        table = Table()
        table.feed(page.read_text(encoding="utf-8"))
        found: List[Tuple[str, List[str]]] = []
        for row in table.rows:
            texts = [t for t, _ in row]
            if len(texts) < 2 or texts[0] in ("津波予報区", "津波予報区名"):
                continue
            name, description = texts[-2], texts[-1]
            covered = [p for p in prefectures if p in description]
            if covered:
                found.append((name, covered))
        return found


def main(sources: Path, output: Path) -> None:
    (output / "prefectures.bin").write_bytes(Boundaries.build(sources / "admin1.geojson"))
    seismic = Areas.seismic(sources / "shindo-name.html")
    (output / "areas.tsv").write_text("".join("\t".join(a) + "\n" for a in seismic), encoding="utf-8")
    prefectures: List[str] = list(dict.fromkeys(a[0] for a in seismic))
    tsunami = Areas.tsunami(sources / "t-yohokuinfo.html", prefectures)
    (output / "tsunami_areas.tsv").write_text("".join(f"{n}\t{','.join(p)}\n" for n, p in tsunami), encoding="utf-8")
    print(f"{len(seismic)} seismic areas, {len(tsunami)} tsunami areas", file=sys.stderr)


if __name__ == "__main__":
    main(Path(sys.argv[1]), Path(sys.argv[2]))
