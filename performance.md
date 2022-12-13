# Rect Test

native:
paint: 40k at 20 FPS
mesh: 160k at 20 FPS

wasm:
paint: 20k at 20 FPS (estimated, since I don't have a working FPS meter)
mesh: 40k at 20 FPS (estimated again)

with precise hit detection (note: this means we're not drawing strokes
on most rectangles):

native:
paint: 80k at 20 FPS (probably because I'm highlighting only one rect)

wasm:
paint: 20k seems pretty smooth, hard to tell exactly how fast it's going
