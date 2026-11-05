import bpy, random, sys
from mathutils import Vector
from pathlib import Path
from math import sin, cos, pi, tau
ROOT = Path(__file__).resolve().parent
sys.path.insert(0,str(ROOT))
import dioramas
import habitantes
NAMES = ['Resort World','Bird World','Venzenulon 7','Alphabetrium',"Unity's Planet"]
SLUGS = ['Resort_World','Bird_World','Venzenulon_7','Alphabetrium','Unitys_Planet']
SUB = ['IMMORTALITY FIELD RESORT','HOME OF BIRDPERSON','THE HIGHLANDS OF BEEBO','REALM OF THE NUMBERICONS','THE ASSIMILATED WORLD']
PAL = [('653057','a34271','dd759c','f9b28e'),('423d52','805466','ba755f','f3b481'),('254953','407259','7b9a54','b4c989'),('1e2858','3950a0','5c83c4','94ebdd'),('256b70','439289','8776b3','c4a9dc')]

# Convierte un color hexadecimal de sRGB a lineal
def rgb(h):
    a = [int(h[i:i+2],16)/255 for i in (0,2,4)]
    return tuple(((v+.055)/1.055)**2.4 if v>.04045 else v/12.92 for v in a)

# Crea o reutiliza un material con brillo metal roca o vidrio
def mat(h, glow=0, metal=0, rock=False, glass=False):
    key = f'{h}_{glow}_{metal}_{rock}_{glass}'
    if key in bpy.data.materials: return bpy.data.materials[key]
    m = bpy.data.materials.new(key); m.diffuse_color = (*rgb(h),1); m.use_nodes = True
    n=m.node_tree.nodes; l=m.node_tree.links; p=n.get('Principled BSDF')
    p.inputs['Base Color'].default_value=(*rgb(h),1)
    p.inputs['Roughness'].default_value=.36 if metal else .68
    p.inputs['Metallic'].default_value=metal
    p.inputs['Emission Color'].default_value=(*rgb(h),1)
    p.inputs['Emission Strength'].default_value=glow
    if rock:
        t=n.new('ShaderNodeTexNoise'); t.inputs['Scale'].default_value=26; t.inputs['Detail'].default_value=4
        b=n.new('ShaderNodeBump'); b.inputs['Strength'].default_value=.31; b.inputs['Distance'].default_value=.09
        l.new(t.outputs['Fac'],b.inputs['Height']); l.new(b.outputs[0],p.inputs['Normal'])
    if glass:
        trans=n.new('ShaderNodeBsdfTransparent'); mix=n.new('ShaderNodeMixShader')
        fres=n.new('ShaderNodeFresnel'); fres.inputs[0].default_value=1.16
        l.new(fres.outputs[0],mix.inputs[0]); l.new(trans.outputs[0],mix.inputs[1])
        l.new(p.outputs[0],mix.inputs[2]); l.new(mix.outputs[0],n.get('Material Output').inputs[0])
    return m

# Nombra el objeto recien creado y le asigna material
def finish(name, m, smooth=False):
    o=bpy.context.object; o.name=name; o.data.materials.append(m)
    if smooth:
        for f in o.data.polygons: f.use_smooth=True
    return o

# Crea una esfera UV
def uv(name, p, scale, m, seg=24, rings=12):
    bpy.ops.mesh.primitive_uv_sphere_add(segments=seg, ring_count=rings, location=p)
    o=finish(name,m,True); o.scale=scale
    return o

# Crea una icoesfera
def ico(name,p,scale,m,sub=1):
    bpy.ops.mesh.primitive_ico_sphere_add(subdivisions=sub, location=p)
    o=finish(name,m); o.scale=scale
    return o

# Crea un cubo
def box(name,p,scale,m):
    bpy.ops.mesh.primitive_cube_add(size=1,location=p)
    o=finish(name,m); o.scale=scale
    return o

# Crea un cilindro o cono entre dos puntos
def beam(name,a,b,r,m,tip=None,n=12):
    a,b=Vector(a),Vector(b); d=b-a
    bpy.ops.mesh.primitive_cone_add(vertices=n,radius1=r,radius2=r if tip is None else tip,depth=d.length,location=(a+b)/2)
    o=finish(name,m,True); o.rotation_euler=d.to_track_quat('Z','Y').to_euler()
    return o

# Crea un tubo que sigue una lista de puntos
def tube(name,pts,r,m,closed=False):
    c=bpy.data.curves.new(name,'CURVE'); c.dimensions='3D'; c.bevel_depth=r; c.bevel_resolution=2
    s=c.splines.new('POLY'); s.points.add(len(pts)-1)
    for p,co in zip(s.points,pts): p.co=(*co,1)
    s.use_cyclic_u=closed
    o=bpy.data.objects.new(name,c); bpy.context.collection.objects.link(o); c.materials.append(m); return o

# Crea un texto 3D
def label(body,p,size,m,rot=None,depth=0):
    c=bpy.data.curves.new(body,'FONT'); c.body=body; c.size=size; c.extrude=depth
    c.bevel_depth=depth*.18; c.space_character=1.13
    o=bpy.data.objects.new(body,c); bpy.context.collection.objects.link(o); o.location=p
    if rot: o.rotation_euler=rot
    c.materials.append(m); return o

# Exporta objetos a un OBJ triangulado para el simulador
def export(path, objects):
    # OBJ triangulado Z arriba: los materiales incluyen el color sRGB para el rasterizador
    deps=bpy.context.evaluated_depsgraph_get()
    with path.open('w',encoding='ascii') as f:
        f.write('# Proyecto Galaxia | Z up | usemtl RRGGBB\n')
        base=1
        for o in objects:
            if o.type not in {'MESH','CURVE','FONT'}: continue
            ev=o.evaluated_get(deps); mesh=ev.to_mesh()
            mesh.calc_loop_triangles()
            for v in mesh.vertices:
                f.write('v %.5f %.5f %.5f\n'%tuple(o.matrix_world@v.co))
            last=None
            for tri in mesh.loop_triangles:
                poly=mesh.polygons[tri.polygon_index]
                m=mesh.materials[poly.material_index] if mesh.materials else None
                name=m.get('flight_tint',m.name[:6]) if m else 'ffffff'
                if last!=name: f.write(f'usemtl {name}\n'); last=name
                f.write('f %d %d %d\n'%tuple(i+base for i in tri.vertices))
            base+=len(mesh.vertices); ev.to_mesh_clear()

# Modela la nave Space Cruiser
def ship():
    gray=mat('687981',0,.65); rim=mat('35464b',0,.52); green=mat('425b51',0,.38)
    light=mat('ffb83a',1.5); cyan=mat('61f3cd',1.3)
    uv('Casco de chatarra',(0,0,-.16),(1.82,1.37,.47),gray,48,20)
    uv('Base de escape',(0,0,-.40),(1.46,1.13,.27),rim,48,18)
    uv('Collar del parabrisas',(0,0,.14),(1.05,.94,.18),green,40,16)
    uv('Burbuja de la cabina',(0,0,.27),(1.02,.88,.78),mat('63bcbf',.08,.2,glass=True),40,20)
    for x in (-.40,.40):
        box('Asiento',(x,.15,.38),(.38,.39,.25),green)
        o=box('Respaldo',(x,.31,.52),(.40,.12,.39),green); o.rotation_euler.x=.13
        uv('Reposacabezas',(x,.32,.72),(.17,.09,.10),rim)
    box('Tablero',(0,-.53,.39),(1.05,.26,.15),rim)
    for j in range(10):
        uv('Indicador',(j*.09-.41,-.54,.47),(.022,.022,.017),cyan,12,6)
    for x in (-1.49,1.49):
        beam('Turbina recuperada',(x,.76,-.10),(x,-.62,-.10),.28,rim)
        for y in (-.54,-.39,-.24,.04,.19,.34,.49):
            beam('Anillo de refrigeracion',(x,y,-.10),(x,y+.045,-.10),.293,gray)
        beam('Escape ionico',(x,.77,-.10),(x,.80,-.10),.202,cyan)
    for a in [j*tau/9 for j in range(9)]:
        tube('Franja ambar',[(r*cos(a),r*.752*sin(a),z) for r,z in [(1.05,.22),(1.29,.165),(1.52,.09),(1.72,-.01),(1.82,-.16),(1.78,-.26)]],.027,light)
    tube('Arco del vidrio',[(0,.89*cos(t*pi/40),.27+.78*sin(t*pi/40)) for t in range(41)],.023,green)
    for x in (-.75,.75):
        beam('Faro',(x,-1.06,-.16),(x,-1.36,-.11),.13,green)
        beam('Lente',(x,-1.36,-.11),(x,-1.39,-.106),.10,light)
    beam('Mástil lateral',(-1,.56,.12),(-1,.56,1.28),.024,gray)
    me=bpy.data.meshes.new('Vela'); me.from_pydata([(-1,.56,1.24),(-1.73,.56,.45),(-1,.56,.49),(-1.46,.47,.69)],[],[(0,1,3),(0,3,2),(3,1,2)])
    o=bpy.data.objects.new('Vela de emergencia',me); bpy.context.collection.objects.link(o)
    me.materials.append(mat('db9161')); me.materials.append(mat('d0cec1')); me.polygons[1].material_index=1

# Prepara render camara luces estrellas y rotulos de una escena
def setup(title,sub,idx,ship_view=False,cut=False):
    sc=bpy.context.scene
    sc.render.engine='BLENDER_EEVEE'; sc.eevee.taa_render_samples=24
    sc.render.resolution_x=3840; sc.render.resolution_y=2160; sc.render.resolution_percentage=100
    sc.render.image_settings.file_format='PNG'; sc.render.image_settings.color_mode='RGB'
    sc.view_settings.view_transform='AgX'
    world=bpy.data.worlds.new('Firmamento'); sc.world=world; world.use_nodes=True
    world.node_tree.nodes.get('Background').inputs[0].default_value=(.014,.022,.047,1)
    world.node_tree.nodes.get('Background').inputs[1].default_value=.45
    aim=Vector((-.97,0,0)) if not ship_view else Vector((-.62,0,.18))
    cam_pos=Vector((6.8,-13,7.7)) if not ship_view else Vector((3.15,-8,3.05))
    if cut:
        cam_pos=Vector((5.6,-11.5,9.5))
        aim=Vector((-1.84,-.32,.0))
    bpy.ops.object.camera_add(location=cam_pos)
    cam=bpy.context.object; cam.rotation_euler=(aim-cam_pos).to_track_quat('-Z','Y').to_euler()
    cam.data.type='ORTHO'; cam.data.ortho_scale=14.9 if not ship_view else 7.65; cam.data.clip_end=190
    if cut: cam.data.ortho_scale=14.25
    sc.camera=cam; cq=cam.rotation_euler.to_quaternion()
    for name,power,color,loc,size in [('Principal',2450,(1,.78,.64),(-4,-6,9),6),('Rebote',950,(.43,.68,1),(5,-6,2),7),('Contraluz',3000,(.43,.78,1),(3,5,5),5)]:
        data=bpy.data.lights.new(name,'AREA'); data.energy=power*(.42 if ship_view else 1); data.color=color; data.shape='DISK'; data.size=size
        o=bpy.data.objects.new(name,data); sc.collection.objects.link(o); o.location=loc; o.rotation_euler=(-Vector(loc)).to_track_quat('-Z','Y').to_euler()
    rnd=random.Random(730+idx)
    verts=[]; faces=[]; indices=[]
    for j in range(890):
        x=rnd.uniform(-9,9); y=rnd.uniform(-5.3,5.3); r=rnd.uniform(.005,.014)
        if j<30: r*=2.5
        p=cam_pos+cq@Vector((x,y,-35-rnd.random()*5)); base=len(verts)
        for dx,dy in [(r,0),(0,r),(-r,0),(0,-r)]: verts.append(p+cq@Vector((dx,dy,0)))
        faces.append((base,base+1,base+2,base+3)); indices.append(j%4)
    me=bpy.data.meshes.new('Estrellas'); me.from_pydata(verts,[],faces)
    o=bpy.data.objects.new('Firmamento | estrellas',me); sc.collection.objects.link(o)
    for h in ['b7c9e4','89b9d8','d7bfcb','637894']: me.materials.append(mat(h,.76))
    for poly,k in zip(me.polygons,indices): poly.material_index=k
    w=cam.data.ortho_scale; x=-w*.455; y=w*.28125
    accent=mat('92d9d6',.76); paper=mat('eaf5ff',.83); dim=mat('9babc8',.57)
    # Coloca un rotulo frente a la camara
    def hud(tx,x,y,sz,m): label(tx,cam_pos+cq@Vector((x,y,-3)),sz,m,cam.rotation_euler)
    hud('P R O Y E C T O   /   G A L A X I A',x,y*.87,w*.012,accent)
    hud(f'{idx+1:02d}   /   ARCHIVO DE VUELO',x,-y*.18,w*.011,accent)
    for j,line in enumerate(title.upper().split('|')):
        hud(line,x,-y*.35-j*w*.041,w*(.032 if len(line)>9 else .040),paper)
    hud(sub,x,-y*.72,w*.010,dim)
    hud('ATMÓSFERA TRANSLÚCIDA  /  UHD' if cut else 'UHD    /    3840 x 2160',x,-y*.88,w*.009,dim)
    for coords in [[(x,y*.80),(x+w*.21,y*.80)],[(x,-y*.77),(x+w*.20,-y*.77)]]:
        tube('Linea editorial',[cam_pos+cq@Vector((a,b,-3)) for a,b in coords],w*.00045,accent)
    for area in bpy.context.screen.areas:
        if area.type=='VIEW_3D':
            area.spaces.active.region_3d.view_perspective='CAMERA'
    return sc

# Genera los archivos de Blender y los renders o exporta los OBJ del simulador
def main():
    mode=sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else []
    export_flight='--exportar-vuelo' in mode
    mode=[m for m in mode if m!='--exportar-vuelo']
    for i,slug in enumerate(SLUGS+['Space_Cruiser','Sol']):
        if mode and slug not in mode: continue
        bpy.ops.wm.read_factory_settings(use_empty=True)
        if i<5: dioramas.cutaway(sys.modules[__name__],i)
        elif i==5:
            ship()
            habitantes.crew(sys.modules[__name__])
        else: dioramas.solar(sys.modules[__name__])
        if export_flight:
            bpy.context.view_layer.update()
            folder=ROOT.parent/'Simulador'/'assets'
            objects=list(bpy.context.scene.objects)
            if i==6:
                for obj in objects:
                    obj.location*=4.3/3
                    obj.scale*=4.3/3
                bpy.context.view_layer.update()
                export(folder/'Sol.obj',objects)
                continue
            glass_prefix=('Atmósfera |','Burbuja de la cabina')
            solid=[obj for obj in objects if not obj.name.startswith(glass_prefix)]
            glass=[obj for obj in objects if obj.name.startswith(glass_prefix)]
            export(folder/f'{slug}.obj',solid)
            export(folder/f'{slug}_vidrio.obj',glass)
            continue
        titles=['Resort|World','Bird|World','Venzenulon|7','Alphabetrium','Unity’s|Planet','Space|Cruiser','Sol']
        subtitles=['MESETA Y CAMPO DE INMORTALIDAD','NIDO Y REFUGIOS DE BIRDPERSON','PICOS, LAGUNAS Y BOSQUE','SALÓN DEL CONSEJO','RED URBANA DE UNITY','NAVE DE RICK Y MORTY','ESTRELLA DEL SISTEMA']
        sc=setup(titles[i],subtitles[i],i,i==5,cut=i<5)
        if i==6:
            tree=sc.compositing_node_group=bpy.data.node_groups.new('Resplandor solar','CompositorNodeTree')
            tree.interface.new_socket(name='Image',in_out='OUTPUT',socket_type='NodeSocketColor')
            layers=tree.nodes.new('CompositorNodeRLayers')
            glare=tree.nodes.new('CompositorNodeGlare'); glare.inputs['Type'].default_value='Fog Glow'
            glare.inputs['Threshold'].default_value=.75
            glare.inputs['Strength'].default_value=.55
            out=tree.nodes.new('NodeGroupOutput')
            tree.links.new(layers.outputs['Image'],glare.inputs['Image'])
            tree.links.new(glare.outputs['Image'],out.inputs['Image'])
        sc.render.filepath='//../4K/'+slug+'.png'
        bpy.context.preferences.filepaths.save_version=0
        bpy.ops.wm.save_as_mainfile(filepath=str(ROOT/'Blender'/f'{slug}.blend'))
        bpy.ops.render.render(write_still=True)
if __name__=='__main__': main()

