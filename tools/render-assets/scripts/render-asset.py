import sys
import os
import argparse
import bpy
import json
import tempfile
import math

from hashlib import sha1
from collections import OrderedDict
from PIL import Image
from pathlib import Path

def fail(message: str):
    print(f"Error: {message}", file=sys.stderr)
    sys.exit(1)

def find(lst, predicate):
    return next((x for x in lst if predicate(x)), None)

def query_children(obj, children):
    for c in obj.children:
        children.append(c) 
        query_children(c, children)

class ArgumentParserBlender(argparse.ArgumentParser):

    def _get_argv_after_doubledash(self):
        try:
            idx = sys.argv.index("--")
            return sys.argv[idx+1:]
        except ValueError as e:
            return []

    def parse_args(self):
        return super().parse_args(args=self._get_argv_after_doubledash())

def main():
    parser = ArgumentParserBlender()
    parser.add_argument('--output', '-o', required=True)
    parser.add_argument('--object-name', '-n', required=True)
    parser.add_argument('--scene', '-s')
    parser.add_argument('--camera-name', '-c')
    parser.add_argument('--n-frames', '-f', type=int, default=32)
    parser.add_argument('--width', '-x', type=int, default=256)
    parser.add_argument('--height', '-y', type=int)

    args = parser.parse_args()

    if args.scene is not None:
        if args.scene not in bpy.data.scenes:
            fail(f"No scene '{args.scene}' found in blend file")
        bpy.context.window.scene = bpy.context.scenes[args.scene]

    cams = list(filter(lambda x: x.type == 'CAMERA', bpy.data.objects))
    if len(cams) == 0:
        fail("No camera found in scene")

    if args.camera_name is not None:
        cam = find(cams, lambda x: x.name == args.camera_name)
        if cam == None:
            fail(f"Camera '{args.camera_name}' not found in scene")
        bpy.context.scene.camera = cam

    obj = find(bpy.data.objects, lambda x: x.type == 'MESH' and x.name == args.object_name)

    if obj == None:
        fail(f"There is no object with name '{args.object_name}'")

    for o in filter(lambda x: x.type == 'MESH', bpy.data.objects):
        o.hide_render = True

    to_hide = [obj]
    query_children(obj, to_hide)
    for x in to_hide:
        x.hide_render = False

    init_angle = obj.rotation_euler[2]
    images = list()

    bpy.context.scene.render.resolution_x = args.width
    bpy.context.scene.render.resolution_y = args.height if args.height is not None else args.width

    print("Rendering...")
    for step in range(args.n_frames):
        with tempfile.NamedTemporaryFile(suffix='.png') as tmp:
            bpy.context.scene.render.filepath = tmp.name
            obj.rotation_euler[2] = init_angle + math.radians(step * 360 / args.n_frames)
            tmp.close()
            bpy.ops.render.render(write_still = True)
            images.append(Image.open(tmp.name))
    print("Rendering complete.")
    print("Merging...")
    widths, heights = zip(*(i.size for i in images))

    total_width = sum(widths)
    max_height = max(heights)

    new_im = Image.new('RGBA', (total_width, max_height))

    x_offset = 0
    for im in images:
        new_im.paste(im, (x_offset,0))
        x_offset += im.size[0]

    os.makedirs(Path(args.output).parent, exist_ok=True)
    new_im.save(args.output)

    print(f"Merging complete. Image saved to {args.output}")

try:
    main()
except Exception as e:
    fail(str(e))
