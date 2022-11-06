import sys
import os
import argparse
import bpy
import json
import tempfile
import math
import traceback

from hashlib import sha1
from collections import OrderedDict
from PIL import Image
from pathlib import Path
from mathutils import Matrix
from mathutils import Euler

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
    parser.add_argument('--z-local-frames', type=int, default=1)
    parser.add_argument('--z-frames', type=int, default=1)
    parser.add_argument('--x-frames', type=int, default=1)
    parser.add_argument('--width', '-x', type=int, default=256)
    parser.add_argument('--height', '-y', type=int)
    parser.add_argument('--no-override', action='store_true')

    args = parser.parse_args()

    if args.no_override and os.path.exists(args.output):
        print(f"Skipping {args.object_name} as {args.output} already exists");
        sys.exit(0)

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

    obj = find(bpy.data.objects, lambda x: (x.type == 'MESH' or x.type == 'EMPTY') and x.name == args.object_name)

    if obj == None:
        fail(f"There is no object with name '{args.object_name}'")

    for o in filter(lambda x: x.type == 'MESH', bpy.data.objects):
        o.hide_render = True

    to_hide = [obj]
    query_children(obj, to_hide)
    for x in to_hide:
        x.hide_render = False

    init_angle_x = obj.rotation_euler[0]
    init_angle_z = obj.rotation_euler[2]

    target_width = args.width
    target_height = args.height if args.height is not None else args.width

    bpy.context.scene.render.resolution_x = target_width
    bpy.context.scene.render.resolution_y = target_height

    x_angle_per_step = 0
    x_rot_offset = 0
    if args.x_frames > 1:
        x_angle_per_step = 180 / (args.x_frames - 1)
        x_rot_offset = -90
    
    ex = obj.rotation_euler[0]
    ey = obj.rotation_euler[1]
    ez = obj.rotation_euler[2]

    images = list()
    bpy.ops.object.select_all(action='DESELECT')

    total_width = args.z_frames * target_width
    total_height = args.x_frames * target_height * args.z_local_frames

    new_im = Image.new('RGBA', (total_width, total_height))

    print("Rendering...")
    block_offset = 0
    for z_local_step in range(args.z_local_frames):
        x_images = list()
        z_local_angle = math.radians(z_local_step * 360 / args.z_local_frames)
        y_offset = 0
        for x_step in range(args.x_frames):
            z_images = list()
            x_angle = math.radians(x_step * x_angle_per_step + x_rot_offset)
            x_offset = 0
            for z_step in range(args.z_frames):
                with tempfile.NamedTemporaryFile(suffix='.png') as tmp:
                    z_angle = math.radians(z_step * 360 / args.z_frames)

                    obj.rotation_euler[0] = ex
                    obj.rotation_euler[1] = ey
                    obj.rotation_euler[2] = ez

                    for c in obj.children:
                        rot_mat = Matrix.Identity(3)
                        rot_mat.rotate(Euler((0.0, 0.0, z_local_angle)))
                        c.rotation_euler = rot_mat.to_euler()


                    rot_mat = Matrix.Identity(3)
                    rot_mat.rotate(Euler((ex, ey, ez)))
                    rot_mat.rotate(Euler((x_angle, 0.0, 0.0)))
                    rot_mat.rotate(Euler((0.0, 0.0, z_angle)))
                    obj.rotation_euler = rot_mat.to_euler()

                    bpy.context.scene.render.filepath = tmp.name
                    tmp.close()
                    bpy.ops.render.render(write_still = True)

                    im = Image.open(tmp.name)
                    new_im.paste(im, (x_offset, block_offset + y_offset))


                x_offset += target_width
            y_offset += target_height
        block_offset += target_height * args.x_frames
    print("Rendering complete.")

    os.makedirs(Path(args.output).parent, exist_ok=True)
    new_im.save(args.output)

    print(f"Merging complete. Image saved to {args.output}")

try:
    main()
except Exception as e:
    traceback.print_exc()
    fail(str(e))
