# Data of the `japan` crate

| File | What | Source | Licence |
| --- | --- | --- | --- |
| `prefectures.bin` | Outlines of the 47 prefectures, simplified (Douglas–Peucker, 0.005°; islands under 0.0005 deg² dropped) | [Natural Earth](https://www.naturalearthdata.com/) 1:10m Admin 1 – States, Provinces | Public domain |
| `areas.tsv` | The 188 areas of earthquake early warnings and intensity reports (細分区域), with their forecast region (府県予報区) and prefecture | JMA, [緊急地震速報や震度情報で用いる区域の名称](https://www.data.jma.go.jp/svd/eqev/data/joho/shindo-name.html) (as of 2024-01-11) | Names of public areas |
| `tsunami_areas.tsv` | The 66 coastal areas of tsunami forecasts (津波予報区) and the prefectures whose coast each covers | JMA, [津波予報区](https://www.data.jma.go.jp/svd/eqev/data/joho/t-yohokuinfo.html) | Names of public areas |

`generate/generate.py` builds all three from the downloaded sources and reproduces the committed
files byte for byte; its docstring says how to run it. Run it again when JMA changes its areas,
and check that the tests of the crate still pass (they pin the counts, 188 and 66).

The Natural Earth outlines are coarse at the scale of a town and do not follow the coast
exactly; `Boundaries::locate_near` therefore accepts a point within a few kilometres of a
coast. They are for maps and for finding the prefecture of a place, not for surveying.
