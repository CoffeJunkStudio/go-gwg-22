import sys
import argparse
import bpy

def fail(message: str):
    print(f"Error: {message}", file=sys.stderr)
    sys.exit(1)

def find(lst, predicate):
    return next((x for x in lst if predicate(x)), None)

class ArgumentParserBlender(argparse.ArgumentParser):

    def _get_argv_after_doubledash(self):
        try:
            idx = sys.argv.index("--")
            return sys.argv[idx+1:]
        except ValueError as e:
            return []

    def parse_args(self):
        return super().parse_args(args=self._get_argv_after_doubledash())

parser = ArgumentParserBlender()
parser.add_argument('--output', '-o', required=True)
parser.add_argument('--object-names', '-n', required=True)

args = parser.parse_args()

obj = find(bpy.data.objects, lambda x: x.name == args.object_name)

if obj == None:
    fail(f"There is no object with name '{args.object_name}'")

for o in filter(lambda x: x.type == 'MESH', bpy.data.objects):
    o.hide_render = True

obj.hide_render = False

bpy.context.scene.render.filepath = args.output

print(f"Rendering {args.output}...")
bpy.ops.render.render(write_still = True)

print(f"Rendering complete.")
