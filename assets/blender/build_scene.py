"""Rebuilds the Swatchbar packaging studio in Blender and renders layer passes.

Run inside Blender (Text editor or the Blender MCP):
    exec(open(".../assets/blender/build_scene.py").read())
    render_scene("tote", ["mask1", "mask2", "light", "gloss"])     # writes <scene>_<pass>.png here

Scenes: duo (bottle + jar), pouch, box, tote.  Passes:
    light  - shading with white products     gloss - reflections only (black products)
    mask1  - R backdrop, G label/paper, B ink (raw coverage)   mask2 - R body, G accent
Artwork textures (label_*.png, art_*.png) come from make_labels.swift / make_art.swift.
"""
import bpy, math, os, numpy as np
from mathutils import Vector

HERE = os.path.dirname(bpy.data.filepath) if bpy.data.filepath else "/Users/forrest/Projects/resources/in_house/swatchbar/assets/blender"
ART = "/Users/forrest/Projects/resources/in_house/swatchbar/assets/blender"
sc = bpy.context.scene

# ---------------------------------------------------------------- clean slate
for o in list(bpy.data.objects): bpy.data.objects.remove(o, do_unlink=True)
for coll in (bpy.data.meshes, bpy.data.materials, bpy.data.lights, bpy.data.cameras):
    for x in list(coll): coll.remove(x)

# ---------------------------------------------------------------- geometry helpers
def smooth(t):
    t = np.clip(t, 0, 1); return t * t * (3 - 2 * t)

def mesh_obj(name, verts, faces, uvs=None, mat_idx=None, smooth_shade=True):
    me = bpy.data.meshes.new(name)
    me.from_pydata(np.asarray(verts).tolist(), [], faces); me.update()
    if uvs is not None:
        uv = me.uv_layers.new(name="UVMap")
        for poly in me.polygons:
            for li in poly.loop_indices:
                uv.data[li].uv = tuple(uvs[me.loops[li].vertex_index])
    for p in me.polygons: p.use_smooth = smooth_shade
    if mat_idx is not None:
        for p, mi in zip(me.polygons, mat_idx): p.material_index = mi
    ob = bpy.data.objects.new(name, me); sc.collection.objects.link(ob); return ob

def lathe(name, prof, segs=192, radial=None):
    prof = [(max(r, 1e-5), z) for r, z in prof]; n = len(prof)
    verts = []; uvs = []
    for j, (r, z) in enumerate(prof):
        for i in range(segs):
            th = 2 * math.pi * i / segs
            rr = r * (1 + radial(th)) if radial else r
            verts.append((rr * math.cos(th), rr * math.sin(th), z)); uvs.append((i / segs, j / (n - 1)))
    faces = []
    for j in range(n - 1):
        for i in range(segs):
            a = j * segs + i; b = j * segs + (i + 1) % segs
            faces.append((a, b, b + segs, a + segs))
    return mesh_obj(name, verts, faces, uvs)

def arc(cx, cz, r, a0, a1, n=12):
    return [(cx + r * math.cos(math.radians(a0 + (a1 - a0) * k / n)), cz + r * math.sin(math.radians(a0 + (a1 - a0) * k / n))) for k in range(n + 1)]

def bez(p0, p1, p2, p3, n=24):
    out = []
    for k in range(n + 1):
        t = k / n; u = 1 - t
        out.append((u**3*p0[0] + 3*u*u*t*p1[0] + 3*u*t*t*p2[0] + t**3*p3[0], u**3*p0[1] + 3*u*u*t*p1[1] + 3*u*t*t*p2[1] + t**3*p3[1]))
    return out

def label_ob(name, r, z0, z1, segs=256):
    verts = []; faces = []; uvs = []
    for j, z in enumerate((z0, z1)):
        for i in range(segs + 1):
            th = -math.pi / 2 + 2 * math.pi * (i / segs - 0.5)   # u=0.5 faces -Y (camera)
            verts.append((r * math.cos(th), r * math.sin(th), z)); uvs.append((i / segs, j))
    for i in range(segs):
        faces.append((i, i + 1, segs + 1 + i + 1, segs + 1 + i))
    return mesh_obj(name, verts, faces, uvs)

def pillow(name, W, H, A, sign, NU=140, NV=200, p=2.0, q=0.6, flat_bot=0.10, seal=True, crink=0.010):
    u = np.linspace(-1, 1, NU + 1); v = np.linspace(0, 1, NV + 1)
    U, V = np.meshgrid(u, v)
    edge = np.clip(1 - np.abs(U)**p, 0, 1)
    g = 0.45 + 0.55 * smooth(V / max(flat_bot, 1e-3)) if flat_bot > 0 else np.ones_like(V)
    if seal: g = g * (1 - 0.94 * smooth((V - 0.80) / 0.10))
    T = A * edge**q * g
    T += crink * np.sin(11 * U + 4 * V) * np.cos(7 * V - 3 * U) * edge * g
    if seal: T += 0.007 * np.sin(V * H * 80) * (V > 0.88) * (edge**0.5)
    x = U * W / 2 * (0.97 + 0.03 * smooth(V / 0.2)); z = V * H; y = -sign * T
    verts = np.stack([x, y, z], -1).reshape(-1, 3)
    faces = []
    for j in range(NV):
        for i in range(NU):
            a = j * (NU + 1) + i
            faces.append((a, a + 1, a + NU + 2, a + NU + 1) if sign > 0 else (a + 1, a, a + NU + 1, a + NU + 2))
    uvs = np.stack([(U + 1) / 2, V], -1).reshape(-1, 2)
    return mesh_obj(name, verts, faces, uvs), (T[0].copy(), x[0].copy())

def strap(name, y, z0=2.85, span=0.75, rise=1.0, n=80, ring=14, wi=0.075, th=0.028):
    verts = []; faces = []
    s = np.linspace(-1, 1, n); px = s * span; pz = z0 + rise * np.cos(s * np.pi / 2)**0.85
    for k in range(n):
        t = np.array([px[min(k + 1, n - 1)] - px[max(k - 1, 0)], pz[min(k + 1, n - 1)] - pz[max(k - 1, 0)]]); t /= np.linalg.norm(t)
        nx, nz = -t[1], t[0]
        for r in range(ring):
            a = 2 * np.pi * r / ring
            verts.append((px[k] + nx * wi * np.cos(a), y + th * np.sin(a), pz[k] + nz * wi * np.cos(a)))
    for k in range(n - 1):
        for r in range(ring):
            a = k * ring + r; b = k * ring + (r + 1) % ring
            faces.append((a, b, b + ring, a + ring))
    return mesh_obj(name, verts, faces)

# ---------------------------------------------------------------- products
# bottle + jar
bp = [(0, 0), (0.30, 0)] + arc(0.30, 0.15, 0.15, -90, 0)[1:] + [(0.45, 2.05)]
bp += bez((0.45, 2.05), (0.45, 2.62), (0.24, 2.55), (0.24, 3.02))[1:] + [(0.24, 3.12), (0.1, 3.12), (0.0, 3.12)]
bottle = lathe("Bottle", bp, 192)
cap_prof = [(0.0, 3.64), (0.20, 3.64)] + arc(0.20, 3.60, 0.075, 90, 0, 6)[1:] + [(0.275, 3.10), (0.262, 3.07), (0.0, 3.07)]
cap = lathe("Cap", cap_prof, 384, radial=lambda th: 0.018 * math.cos(44 * th))
label_ob("BottleLabel", 0.454, 0.75, 1.86)
lathe("BottleBandLow", [(0.456, 0.66), (0.456, 0.75)]); lathe("BottleBandHigh", [(0.456, 1.86), (0.456, 1.95)])
jp = [(0, 0), (0.60, 0)] + arc(0.60, 0.15, 0.15, -90, 0)[1:] + [(0.75, 1.22), (0.72, 1.26), (0.0, 1.26)]
lathe("Jar", jp, 256)
lp = [(0.0, 1.92), (0.68, 1.92)] + arc(0.68, 1.84, 0.08, 90, 0, 6)[1:] + [(0.76, 1.30), (0.745, 1.26), (0.0, 1.26)]
lathe("Lid", lp, 256)
label_ob("JarLabel", 0.754, 0.30, 1.04)
lathe("JarBandLow", [(0.756, 0.21), (0.756, 0.30)]); lathe("JarBandHigh", [(0.756, 1.04), (0.756, 1.13)])
for n in ("Bottle", "Cap", "BottleLabel", "BottleBandLow", "BottleBandHigh"): bpy.data.objects[n].location = (-1.0, 0.15, 0)
for n in ("Jar", "Lid", "JarLabel", "JarBandLow", "JarBandHigh"): bpy.data.objects[n].location = (1.45, -0.55, 0)

# pouch
pillow("PouchFront", 2.2, 3.2, 0.42, +1)
_, (T0, X0) = pillow("PouchBack", 2.2, 3.2, 0.42, -1)
NU = 140; rows = 8; verts = []; faces = []
for r in range(rows + 1):
    t = r / rows
    for i in range(NU + 1): verts.append((X0[i], (-T0[i]) * (1 - 2 * t), 0.0))
for r in range(rows):
    for i in range(NU):
        a = r * (NU + 1) + i; faces.append((a, a + 1, a + NU + 2, a + NU + 1))
mesh_obj("PouchBase", verts, faces)

# box
bx, by, h = 1.2, 0.55, 3.2
F = [([(-bx, -by, 0), (bx, -by, 0), (bx, -by, h), (-bx, -by, h)], 0), ([(bx, -by, 0), (bx, by, 0), (bx, by, h), (bx, -by, h)], 1),
     ([(-bx, by, 0), (-bx, -by, 0), (-bx, -by, h), (-bx, by, h)], 2), ([(bx, by, 0), (-bx, by, 0), (-bx, by, h), (bx, by, h)], 2),
     ([(-bx, -by, h), (bx, -by, h), (bx, by, h), (-bx, by, h)], 2), ([(-bx, by, 0), (bx, by, 0), (bx, -by, 0), (-bx, -by, 0)], 2)]
verts = []; faces = []; mats = []
for vs, mi in F:
    k = len(verts); verts += vs; faces.append((k, k + 1, k + 2, k + 3)); mats.append(mi)
box = mesh_obj("Box", verts, faces, None, mats)
uv = box.data.uv_layers.new(name="UVMap")
for poly in box.data.polygons:
    for n, li in enumerate(poly.loop_indices): uv.data[li].uv = [(0, 0), (1, 0), (1, 1), (0, 1)][n]
bev = box.modifiers.new("Bevel", 'BEVEL'); bev.width = 0.05; bev.segments = 4; bev.limit_method = 'ANGLE'
try: bev.harden_normals = True
except Exception: pass
bpy.context.view_layer.objects.active = box
bpy.ops.object.modifier_apply(modifier="Bevel")
box.rotation_euler.z = math.radians(-28)

# tote
pillow("ToteBody", 2.6, 2.9, 0.16, +1, p=6.0, q=0.5, flat_bot=0.04, seal=False, crink=0.006)
pillow("ToteBack", 2.6, 2.9, 0.16, -1, p=6.0, q=0.5, flat_bot=0.04, seal=False, crink=0.006)
strap("ToteHandleF", -0.19); strap("ToteHandleB", +0.19)

groups = {"pouch": ["PouchFront", "PouchBack", "PouchBase"], "box": ["Box"], "tote": ["ToteBody", "ToteBack", "ToteHandleF", "ToteHandleB"],
          "duo": ["Bottle", "Cap", "BottleLabel", "BottleBandLow", "BottleBandHigh", "Jar", "Lid", "JarLabel", "JarBandLow", "JarBandHigh"]}
for n in groups["pouch"] + groups["tote"] + groups["box"]: bpy.data.objects[n].location.x = 0.35

# props: smaller copies sharing the hero's mesh (and so its materials), staged behind for depth
def clone_group(scene, names, tag, loc, rot_z, scale):
    root = bpy.data.objects.new(f"Root_{tag}", None); sc.collection.objects.link(root)
    root.location = loc; root.rotation_euler.z = math.radians(rot_z); root.scale = (scale,) * 3
    for n in names:
        src = bpy.data.objects[n]
        c = bpy.data.objects.new(f"{n}__{tag}", src.data); sc.collection.objects.link(c)
        base = bpy.data.objects[names[0]].location
        c.parent = root; c.location = (src.location.x - base.x, src.location.y - base.y, src.location.z); c.rotation_euler = src.rotation_euler.copy()
        groups[scene].append(c.name)
HERO_X = {"duo": 0.0, "pouch": 0.35, "box": 0.35, "tote": 0.35}
HERO = {k: list(v) for k, v in groups.items()}   # snapshot: clones must copy the hero only
PROPS = {
    "duo":   [("a", (-3.15, 1.9, 0), 8, 0.62, ("Bottle", "Cap", "BottleLabel", "BottleBandLow", "BottleBandHigh")),
              ("b", (3.55, 1.5, 0), -10, 0.55, ("Jar", "Lid", "JarLabel", "JarBandLow", "JarBandHigh"))],
    "pouch": [("a", (-2.25, 1.3, 0), 14, 0.72, list(groups["pouch"])), ("b", (3.1, 1.0, 0), -12, 0.62, list(groups["pouch"]))],
    "box":   [("a", (-2.55, 1.3, 0), -10, 0.66, list(groups["box"])), ("b", (3.05, 0.2, 0), 18, 0.45, list(groups["box"]))],
    "tote":  [("a", (-2.5, 1.5, 0), 9, 0.72, list(groups["tote"])), ("b", (3.0, 1.7, 0), -9, 0.58, list(groups["tote"]))],
}
for scene, plist in PROPS.items():
    for tag, loc, rz, k, names in plist:
        clone_group(scene, list(names), f"{scene}_{tag}", loc, rz, k)

# backdrop sweep
pts = [(-14, 0), (1.5, 0)] + arc(1.5, 3.0, 3.0, -90, 0, 24)[1:] + [(4.5, 14)]
verts = []; faces = []
for j, (y, z) in enumerate(pts):
    for x in (-18, 18): verts.append((x, y, z))
for j in range(len(pts) - 1):
    a = 2 * j; faces.append((a, a + 1, a + 3, a + 2))
mesh_obj("Backdrop", verts, faces)

# ---------------------------------------------------------------- camera & lights
cd = bpy.data.cameras.new("Cam"); cam = bpy.data.objects.new("Cam", cd); sc.collection.objects.link(cam)
cd.lens = 85; cd.sensor_width = 36
cam.location = (0.35, -18.5, 1.95)
cam.rotation_euler = (Vector((0.35, 0, 1.95)) - cam.location).to_track_quat('-Z', 'Y').to_euler()
sc.camera = cam
def area(name, loc, size, energy, aim=(0, 0, 1.5), shape='SQUARE', size_y=None):
    d = bpy.data.lights.new(name, 'AREA'); d.energy = energy; d.size = size; d.shape = shape
    if size_y: d.size_y = size_y
    o = bpy.data.objects.new(name, d); sc.collection.objects.link(o); o.location = loc
    o.rotation_euler = (Vector(aim) - Vector(loc)).to_track_quat('-Z', 'Y').to_euler()
area("Key", (-6, -8, 7), 7, 330); area("Fill", (7, -9, 3), 6, 18); area("Rim", (3, 5, 6), 4, 150)
area("Strip", (5.5, -7, 2.2), 0.8, 140, aim=(0.3, 0, 1.8), shape='RECTANGLE', size_y=6.0)

glow = bpy.data.lights.new("WallGlow", 'SPOT'); glow.energy = 1400; glow.spot_size = math.radians(58); glow.spot_blend = 0.9; glow.shadow_soft_size = 1.5
go = bpy.data.objects.new("WallGlow", glow); sc.collection.objects.link(go); go.location = (-3.5, -6.0, 6.5)
go.rotation_euler = (Vector((0.6, 4.4, 2.0)) - Vector(go.location)).to_track_quat('-Z', 'Y').to_euler()

# ---------------------------------------------------------------- materials
def new_mat(name):
    m = bpy.data.materials.new(name); m.use_nodes = True; nt = m.node_tree
    for n in list(nt.nodes): nt.nodes.remove(n)
    return m, nt, nt.nodes.new("ShaderNodeOutputMaterial")

def principled(name, base, rough, spec=0.5, bump=None):
    """bump = (texture file, (scale_u, scale_v), strength): a height map for surface relief."""
    m, nt, out = new_mat(name); p = nt.nodes.new("ShaderNodeBsdfPrincipled")
    p.inputs["Base Color"].default_value = (*base, 1); p.inputs["Roughness"].default_value = rough
    if "Specular IOR Level" in p.inputs: p.inputs["Specular IOR Level"].default_value = spec
    if bump:
        fn, (su, sv), strength = bump
        uvn = nt.nodes.new("ShaderNodeTexCoord"); mp = nt.nodes.new("ShaderNodeMapping"); mp.inputs["Scale"].default_value = (su, sv, 1)
        t = nt.nodes.new("ShaderNodeTexImage"); t.image = tex_image(os.path.join("tex", fn)); t.interpolation = "Cubic"; t.extension = "REPEAT"
        b = nt.nodes.new("ShaderNodeBump"); b.inputs["Strength"].default_value = strength; b.inputs["Distance"].default_value = 0.02
        nt.links.new(uvn.outputs["UV"], mp.inputs["Vector"]); nt.links.new(mp.outputs["Vector"], t.inputs["Vector"])
        nt.links.new(t.outputs["Color"], b.inputs["Height"]); nt.links.new(b.outputs["Normal"], p.inputs["Normal"])
    nt.links.new(p.outputs[0], out.inputs[0]); return m

def tex_image(tex):
    im = bpy.data.images.load(os.path.join(ART, tex), check_existing=True); im.colorspace_settings.name = "Non-Color"; return im

def flat_emission(name, color):
    m, nt, out = new_mat(name); e = nt.nodes.new("ShaderNodeEmission"); e.inputs["Color"].default_value = (*color, 1)
    nt.links.new(e.outputs[0], out.inputs[0]); return m

def label_emission(name, tex):         # bottle/jar label: R = ink, paper = 1 - R
    m, nt, out = new_mat(name); e = nt.nodes.new("ShaderNodeEmission")
    t = nt.nodes.new("ShaderNodeTexImage"); t.image = tex_image(tex); t.interpolation = "Cubic"; t.extension = "EXTEND"
    sep = nt.nodes.new("ShaderNodeSeparateColor"); nt.links.new(t.outputs["Color"], sep.inputs["Color"])
    comb = nt.nodes.new("ShaderNodeCombineColor"); inv = nt.nodes.new("ShaderNodeMath"); inv.operation = "SUBTRACT"; inv.inputs[0].default_value = 1.0
    nt.links.new(sep.outputs["Red"], inv.inputs[1]); nt.links.new(inv.outputs[0], comb.inputs["Green"]); nt.links.new(sep.outputs["Red"], comb.inputs["Blue"])
    nt.links.new(comb.outputs[0], e.inputs["Color"]); nt.links.new(e.outputs[0], out.inputs[0]); return m

def art_emission(name, tex, mode):      # art3: R ink, G accent, B paper, rest body
    m, nt, out = new_mat(name); e = nt.nodes.new("ShaderNodeEmission")
    t = nt.nodes.new("ShaderNodeTexImage"); t.image = tex_image(tex); t.interpolation = "Cubic"; t.extension = "EXTEND"
    sep = nt.nodes.new("ShaderNodeSeparateColor"); nt.links.new(t.outputs["Color"], sep.inputs["Color"])
    comb = nt.nodes.new("ShaderNodeCombineColor"); R, G, B = sep.outputs["Red"], sep.outputs["Green"], sep.outputs["Blue"]
    if mode == "mask1":
        nt.links.new(B, comb.inputs["Green"]); nt.links.new(R, comb.inputs["Blue"])
    else:
        s1 = nt.nodes.new("ShaderNodeMath"); s1.operation = "ADD"; nt.links.new(R, s1.inputs[0]); nt.links.new(G, s1.inputs[1])
        s2 = nt.nodes.new("ShaderNodeMath"); s2.operation = "ADD"; nt.links.new(s1.outputs[0], s2.inputs[0]); nt.links.new(B, s2.inputs[1])
        inv = nt.nodes.new("ShaderNodeMath"); inv.operation = "SUBTRACT"; inv.use_clamp = True; inv.inputs[0].default_value = 1.0
        nt.links.new(s2.outputs[0], inv.inputs[1]); nt.links.new(inv.outputs[0], comb.inputs["Red"]); nt.links.new(G, comb.inputs["Green"])
    nt.links.new(comb.outputs[0], e.inputs["Color"]); nt.links.new(e.outputs[0], out.inputs[0]); return m

sets = {"light": {}, "gloss": {}, "mask1": {}, "mask2": {}}
W = (1, 1, 1); K = (0, 0, 0)
def add(name, rough, spec, slots, mask1=None, mask2=None, bump=None):
    for mode, base in (("light", W), ("gloss", K)):
        sets[mode][name] = [principled(f"{mode}_{name}_{i}", base, rough, spec, bump) for i, _ in enumerate(slots)]
    for mode, fixed in (("mask1", mask1), ("mask2", mask2)):
        sets[mode][name] = [(art_emission(f"{mode}_{name}_{i}", s, mode) if s else flat_emission(f"{mode}_{name}_{i}", fixed or (0, 0, 0))) for i, s in enumerate(slots)]

# bottle / jar (flat colors; labels carry ink as R)
for n, (r, s) in {"Bottle": (0.34, 0.5), "Cap": (0.36, 0.5), "BottleBandLow": (0.34, 0.5), "BottleBandHigh": (0.34, 0.5),
                  "Jar": (0.30, 0.5), "Lid": (0.32, 0.5), "JarBandLow": (0.3, 0.5), "JarBandHigh": (0.3, 0.5)}.items():
    body = n in ("Bottle", "Cap", "Jar", "Lid")
    add(n, r, s, [None], mask1=(0, 0, 0), mask2=(1, 0, 0) if body else (0, 1, 0))
for n, tex in (("BottleLabel", "label_bottle.png"), ("JarLabel", "label_jar.png")):
    for mode, base in (("light", W), ("gloss", K)): sets[mode][n] = [principled(f"{mode}_{n}", base, 0.5, 0.4)]
    sets["mask1"][n] = [label_emission("m1_" + n, tex)]; sets["mask2"][n] = [flat_emission("m2_" + n, (0, 0, 0))]
add("Backdrop", 0.85, 0.2, [None], mask1=(1, 0, 0), mask2=(0, 0, 0))
# pouch / box / tote
FOIL = ("foil.png", (1.6, 2.2), 0.30)
add("PouchFront", 0.30, 0.5, ["art_pouch.png"], bump=FOIL); add("PouchBack", 0.30, 0.5, [None], mask2=(1, 0, 0), bump=FOIL); add("PouchBase", 0.30, 0.5, [None], mask2=(1, 0, 0))
PAPER = ("paper.png", (5, 7), 0.12)
add("Box", 0.50, 0.4, ["art_box_front.png", "art_box_side.png", None], mask2=(1, 0, 0), bump=PAPER)
CANVAS = ("canvas.png", (26, 29), 0.45)
add("ToteBody", 0.88, 0.15, ["art_tote.png"], bump=CANVAS); add("ToteBack", 0.88, 0.15, [None], mask2=(1, 0, 0), bump=CANVAS)
add("ToteHandleF", 0.9, 0.15, [None], mask2=(1, 0, 0), bump=("canvas.png", (60, 6), 0.4)); add("ToteHandleB", 0.9, 0.15, [None], mask2=(1, 0, 0), bump=("canvas.png", (60, 6), 0.4))

# ---------------------------------------------------------------- render
if not sc.world: sc.world = bpy.data.worlds.new("World")
sc.world.use_nodes = True
sc.render.engine = "CYCLES"; sc.cycles.device = "CPU"
sc.render.resolution_x, sc.render.resolution_y, sc.render.resolution_percentage = 2240, 1260, 100
sc.render.image_settings.file_format = "PNG"; sc.render.image_settings.color_depth = "8"; sc.render.image_settings.color_mode = "RGB"
sc.view_settings.look = "None"; sc.view_settings.exposure = 0; sc.view_settings.gamma = 1; sc.render.film_transparent = False

def show(scene):
    for g, names in groups.items():
        for n in names:
            o = bpy.data.objects[n]; o.hide_render = (g != scene); o.hide_viewport = (g != scene)

def setmode(mode):
    for n, mats in sets[mode].items():
        ob = bpy.data.objects[n]; ob.data.materials.clear()
        for m in mats: ob.data.materials.append(m)
    bg = sc.world.node_tree.nodes.get("Background")
    if mode.startswith("mask"):
        bg.inputs["Strength"].default_value = 0.0
        sc.view_settings.view_transform = "Raw"; sc.cycles.samples = 24; sc.cycles.use_denoising = False; sc.cycles.max_bounces = 0
    else:
        bg.inputs["Color"].default_value = (1, 1, 1, 1); bg.inputs["Strength"].default_value = 0.10
        sc.view_settings.view_transform = "Standard"; sc.cycles.samples = 80; sc.cycles.use_denoising = True; sc.cycles.max_bounces = 6

def render_scene(scene, passes=("mask1", "mask2", "light", "gloss")):
    show(scene)
    for mode in passes:
        setmode(mode)
        sc.render.filepath = f"{ART}/{'duo' if scene == 'duo' else scene}_{mode}.png"
        bpy.ops.render.render(write_still=True)
