from mathutils import Vector
from math import sin, cos, sqrt, exp, pi, tau
import random
import bpy
import habitantes

# Crea un objeto de malla con sus materiales
def model(name, verts, faces, materials, indices=None):
    mesh=bpy.data.meshes.new(name); mesh.from_pydata(verts,[],faces); mesh.update()
    obj=bpy.data.objects.new(name,mesh); bpy.context.collection.objects.link(obj)
    for m in materials: mesh.materials.append(m)
    if indices:
        for p,j in zip(mesh.polygons,indices): p.material_index=j
    return obj

# Construye un mundo completo: hemisferio terreno atmosfera y objetos
def cutaway(g,i):
    mat,uv,ico,beam,tube,box,label=(getattr(g,k) for k in ('mat','uv','ico','beam','tube','box','label'))
    # /// Semilla de distribucion y requiere Blender
    rnd=random.Random(9710+i)
    palettes=[
        ('482d59','713c69','ab5984','d385a8','eeb2ad'),
        ('362942','594054','925a5c','bb7868','edac85'),
        ('263f49','416360','63816a','92a679','b6bb8f'),
        ('2b333b','3d4947','566151','747e63','b8aa75'),
        ('383656','51476e','776694','a38abd','c2a9d6'),
    ]
    palette=palettes[i]
    rock=[mat(h,rock=True) for h in palette]
    n=128; rings=30; radius=3.23
    # Radio ondulado del borde del hemisferio
    def edge(a): return radius*(1+.023*sin(5*a+i)+.017*cos(9*a-i))
    verts=[]; faces=[]; colors=[]
    for j in range(rings+1):
        t=(pi*.5-.0003)*j/rings
        for k in range(n):
            a=tau*k/n
            r=edge(a)*cos(t)*(1+.025*sin(11*a+8*t)*sin(t))
            verts.append((r*cos(a),r*sin(a),-radius*sin(t)))
    for j in range(rings):
        for k in range(n):
            a=j*n+k; b=j*n+(k+1)%n
            faces.append((a,a+n,b+n,b))
            colors.append([4,3,2,3,1,2,1,0,1,0][min(9,int(j/3+.44+.50*sin(k*tau*4/n+j*.41)))])
    faces.append(tuple(reversed(range(rings*n,(rings+1)*n)))); colors.append(0)
    shell=model('Hemisferio | estratos expuestos',verts,faces,rock,colors)
    for f in shell.data.polygons: f.use_smooth=True

    atmosphere(g,i,edge,radius)

    # /// Relieve de los planetas y requiere Blender
    def height(x,y):
        r=sqrt(x*x+y*y)
        waves=.10*sin(x*2.9+y*1.9+i)+.074*cos(y*4.1-x*1.7)
        peaks=[.53*exp(-((x+.4)**2+(y-.62)**2)*1.3),
               .66*exp(-((x-.52)**2+(y-1.1)**2)*.8),
               .70*exp(-((x+.9)**2+(y-.95)**2)*1.15),
               .21*exp(-((x-.25)**2+(y-.66)**2)*1.6),
               .34*exp(-((x-.2)**2+(y-.86)**2)*1.2)][i]
        return (.16+waves+peaks)*max(0,min(1,(3.06-r)*1.9))
    top_pal=[('bd789c','da9aab','f0b7a5'),('a66868','d79478','e3aa89'),
             ('52866f','819953','a6b673'),('47574b','657662','8b8d68'),
             ('488f92','68b5a8','a994c4')][i]
    ground=[mat(h,rock=True) for h in top_pal]
    verts=[(0,0,height(0,0))]; faces=[]; colors=[]
    for j in range(1,35):
        for k in range(n):
            a=tau*k/n; r=edge(a)*j/34
            x=r*cos(a); y=r*sin(a)
            verts.append((x,y,height(x,y)))
    # Elige el tipo de suelo de un punto
    def soil(x,y):
        w=.59*sin(1.7*x+1.7*y+i)+.29*cos(y*3.9-x*1.4)+.12*sin(x*8-y*5)
        return 0 if w<-.31 else 1 if w<.39 else 2
    for k in range(n):
        faces.append((0,1+k,1+(k+1)%n)); colors.append(1)
    for j in range(33):
        for k in range(n):
            a=1+j*n+k; b=1+j*n+(k+1)%n
            faces.append((a,a+n,b+n,b)); colors.append(soil(*verts[a][:2]))
    terrain=model('Superficie | terreno del diorama',verts,faces,ground,colors)
    for f in terrain.data.polygons: f.use_smooth=True
    tube('Borde del corte',[(edge(a)*cos(a),edge(a)*sin(a),.006) for a in (k*tau/n for k in range(n))],.027,rock[-1],True)
    for t,m in [(.18,rock[3]),(.39,rock[3]),(.70,rock[2]),(.96,rock[1])]:
        points=[]
        for k in range(n):
            a=k*tau/n; u=t+.013*sin(a*4+t*5); r=edge(a)*cos(u)*(1+.025*sin(11*a+8*u)*sin(u))
            points.append(((r+.009)*cos(a),(r+.009)*sin(a),-radius*sin(u)))
        tube('Veta geologica',points,.014,m,True)

    gold=mat('eac180',.09,.35); aqua=mat('80f0de',1.4); cream=mat('ffe4bc',.08)
    city=mat('695989',0,.40); trail=mat(top_pal[2])
    # Crea un camino pegado al terreno
    def road(name,pts,thickness=.03,material=trail):
        tube(name,[(x,y,height(x,y)+.034) for x,y in pts],thickness,material)
    # Puntos de una circunferencia
    def circle(x,y,r,count=80):
        return [(x+r*cos(a),y+r*sin(a)) for a in (k*tau/count for k in range(count+1))]
    # Crea un arbol: tronco y tres copas
    def tree(x,y,h,pal):
        z=height(x,y)
        beam('Tronco',(x,y,z),(x,y,z+h*.66),h*.095,mat('584452'),h*.045,7)
        for k in range(3):
            a=k*2.4; r=h*(.15 if k else .04)
            ico('Follaje',(x+r*cos(a),y+r*sin(a),z+h*(.60+k*.10)),(h*.38,h*.34,h*.48),mat(rnd.choice(pal)),2)
    # Crea una torre con franjas iluminadas
    def tower(x,y,z,r,h,body,window,levels=4):
        beam('Torre',(x,y,z),(x,y,z+h),r,body,r*.66,12)
        for k in range(1,levels+1):
            u=k/(levels+1)
            beam('Franja iluminada',(x,y,z+h*u),(x,y,z+h*u+.025),r*(1-.34*u)+.008,window,n=12)
        uv('Remate',(x,y,z+h),(r*.67,r*.67,r*.42),body,16,8)
    # Reparte objetos al azar con semilla fija sobre el terreno
    def scatter(count,allow,fn):
        for k in range(count):
            a=rnd.random()*tau; r=2.88*sqrt(rnd.random()); x=r*cos(a); y=r*sin(a)
            if allow(x,y): fn(x,y)
    if i==0:
        cx,cy=-.26,.48; base=.86
        beam('Meseta del resort',(cx,cy,.12),(cx,cy,base),1.68,rock[3],1.42,36)
        beam('Plaza del resort',(cx,cy,base),(cx,cy,base+.075),1.37,gold,1.37,64)
        uv('Laguna termal',(cx-.33,cy-.48,base+.076),(.49,.31,.014),mat('5bd8d1',.45,.28),40,12)
        for k in range(15):
            a=k*2.4; r=.27*sqrt(k); x=cx+r*cos(a); y=cy+r*sin(a)+.17
            if (x-cx+.33)**2+(y-cy+.48)**2<.32: continue
            tower(x,y,base+.08,.098+rnd.random()*.07,.28+rnd.random()*.64,gold,aqua,3)
        for side in (-1,1):
            points=[]
            for k in range(160):
                a=k*tau/159
                points.append((cx+1.18*cos(a),cy-.12+side*.15+.66*sin(a),base+.26+.21*sin(2*a)+.105*cos(a)))
            tube('Whirly Dirly | vias',points,.031,cream)
        for k in range(14):
            a=k*tau/14
            x=cx+1.18*cos(a); y=cy-.12+.66*sin(a)
            beam('Soporte del Whirly Dirly',(x,y,base+.08),(x,y,base+.26+.21*sin(2*a)+.105*cos(a)),.024,gold)
        for rad in (1.45,1.50):
            tube('Anillo del campo de inmortalidad',[(x,y,base+.10) for x,y in circle(cx,cy,rad)],.017,aqua)
        for angle in (0,.8,1.6,2.4):
            tube('Arco del campo de inmortalidad',[(cx+1.45*cos(t)*cos(angle),cy+1.45*cos(t)*sin(angle),base+.10+1.40*sin(t)) for t in (k*pi/64 for k in range(65))],.009,mat('82ced2',.55))
        for k in range(8):
            a=3.35+k*.31; x=2.45*cos(a); y=2.40*sin(a); z=height(x,y)
            tower(x,y,z,.115,.18+.054*(k%4),gold,aqua,2)
            if k%2==0:
                uv('Piscina satélite',(x+.18,y+.09,z+.027),(.18,.11,.02),mat('68bfc8',.3),24,8)
        road('Paseo exterior',[(2.30*cos(a),2.24*sin(a)) for a in (3.25+k*.032 for k in range(88))],.029,cream)
        scatter(118,lambda x,y:(x-cx)**2+(y-cy)**2>2.43,lambda x,y:tree(x,y,rnd.uniform(.19,.47),['e876a2','a34b9b','c16291','eeb2b2']))
    elif i==1:
        for k in range(26):
            a=k*2.399; r=.31*sqrt(k); x=r*cos(a)-.19; y=r*sin(a)+.51
            z=height(x,y)-.12; h=rnd.uniform(.38,1.36); w=rnd.uniform(.17,.31)
            beam('Aguja de roca',(x,y,z),(x,y,z+h),w,rock[k%4+1],w*.72,7)
            for step in (.27,.52,.73):
                z0=z+h*step; s=w*(1-.28*step)+.012
                beam('Banda sedimentaria',(x,y,z0),(x,y,z0+.048),s,rock[1],s*.98,7)
        nx,ny,nz=.52,.25,1.38
        beam('Pilar del nido',(nx,ny,height(nx,ny)),(nx,ny,nz),.54,rock[2],.40,9)
        for k in range(6):
            r=.42+k*.024
            tube('Ramas entretejidas',[(nx+r*cos(a),ny+r*sin(a),nz+k*.034+.05*sin(5*a+k)) for a in (j*tau/96 for j in range(96))],.038,mat(['8d5857','a07158','563947'][k%3]),True)
        for k in range(5):
            a=k*tau/5
            uv('Huevos del nido',(nx+.24*cos(a),ny+.23*sin(a),nz+.25),(.107,.107,.169),cream,20,10)
        for k in range(4):
            x=-1.88+k*.66; y=-.94+.16*(k%2); z=height(x,y)
            beam('Refugio elevado',(x,y,z),(x,y,z+.40),.13,rock[1])
            uv('Refugio oval',(x,y,z+.51),(.24,.22,.26),mat('d58d69'),20,10)
            uv('Entrada del refugio',(x,y-.195,z+.51),(.092,.039,.127),mat('442c3d'),16,8)
        road('Sendero hacia los refugios',[(x,-1.25+.10*sin(x*4)) for x in [k/22 for k in range(-49,31)]],.038)
        timber=mat('885d4c',rock=True); straw=mat('c18b65',rock=True)
        for j in range(9):
            a=3.1+j*.27; x=2.27*cos(a); y=2.12*sin(a)
            z=height(x,y)
            box('Casa del poblado',(x,y,z+.16),(.32,.30,.33),timber)
            uv('Techo de madera',(x,y,z+.32),(.24,.22,.14),straw,14,7)
            box('Puerta del poblado',(x,y-.154,z+.123),(.092,.027,.18),mat('46323c'))
        for j in range(14):
            x=-1.37+j*.195; y=-1.61+.08*sin(j*.31)
            box('Tablones del puente',(x,y,height(x,y)+.12),(.17,.30,.048),timber)
    elif i==2:
        for k in range(28):
            a=k*2.399; r=.31*sqrt(k); x=-.53+r*cos(a); y=.94+.56*r*sin(a)
            z=height(x,y); h=rnd.uniform(.25,.98)*(1+.43*(y>.9))
            beam('Pico de Venzenulon',(x,y,z-.12),(x+.04,y+.07,z+h),rnd.uniform(.18,.41),mat(rnd.choice(['697d83','91a6a7','536f78'])),.023,5)
            if h>.85:
                beam('Cresta clara',(x+.027,y+.047,z+h*.65),(x+.04,y+.07,z+h+.012),h*.105,mat('d2d6bf'),.012,5)
        for k in range(3):
            x=.95+.39*k; y=-.49-.15*k
            uv('Pozas del valle',(x,y,height(x,y)+.035),(.39,.24,.046),mat('70d0d0',.28,.25),32,12)
        scatter(105,lambda x,y:y<.48 and not (x>.59 and -.93<y<-.12),lambda x,y:tree(x,y,rnd.uniform(.16,.39),['82ad73','587d68','a9bf77']))
        for k in range(24):
            t=k/23; x=1.02+.98*t+.17*sin(t*11); y=-.45-1.51*t
            z=height(x,y)
            uv('Cauce de montaña',(x,y,z+.027),(.138,.13,.034),mat('68b8b7',.18),16,8)
            if k%3==0:
                ico('Roca del cauce',(x-.17,y,z+.049),(.10,.10,.09),mat('708780'),1)
        for k in range(6):
            x=-1.53+k*.52; y=-.31+.18*sin(k); z=.99+.11*(k%3)
            tube('Bandada azul',[(x-.19,y,z+.074),(x-.085,y-.05,z),(x,y-.06,z+.019),(x+.11,y,z+.092)],.03,mat('6acfe8',.27))
    elif i==3:
        # Salon abierto para mostrar el eje de la alfombra y el consejo desde arriba
        dark=mat('354843',0,.25); stone=mat('6e8071',0,.18)
        gilt=mat('c6a05b',.13,.55); carpet=mat('8c263e',.025)
        plinth=.43
        box('Cimiento rectangular',(0,.31,.21),(3.54,2.89,.43),dark)
        box('Pavimento del salón',(0,.31,plinth),(3.46,2.80,.09),stone)
        box('Alfombra ceremonial',(0,.12,plinth+.055),(.74,2.40,.022),carpet)
        for sign in (-1,1):
            x=sign*1.55
            for j in range(5):
                y=-.84+.46*j
                box('Basa del pilar',(x,y,.54),(.30,.31,.18),gilt)
                box('Pilar del salón',(x,y,.97),(.19,.23,.85),dark)
                box('Filete vertical',(x-sign*.102,y-.12,.98),(.023,.018,.83),gilt)
                box('Capitel dorado',(x,y,1.39),(.28,.29,.10),gilt)
                if j<4:
                    arch=[(x,y+.23+.23*cos(t),1.39+.25*sin(t)) for t in (k*pi/28 for k in range(29))]
                    tube('Arcada lateral',arch,.046,gilt)
        for j in range(3):
            box('Escalón de acceso',(0,-1.15-j*.12,.37-j*.083),(1.8+j*.13,.22,.12),stone)
        for j in range(2):
            box('Tarima del consejo',(0,1.08+j*.095,.50+j*.08),(2.85-j*.24,.78-j*.17,.13),dark)
        for j,(letter,h) in enumerate(zip('JQF',['70e09b','ff9851','8acde9'])):
            x=(j-1)*.74; y=1.13; z=.64
            box('Pedestal del consejero',(x,y,z),(.41,.42,.15),gilt)
            label(letter,(x-.19,y-.14,z+.09),.57,mat(h,.36,.20),(pi/2,0,0),.073)
            beam('Bastón elemental',(x+.25,y,z+.12),(x+.25,y,z+.65),.018,gilt)
            uv('Gema elemental',(x+.25,y,z+.65),(.053,.053,.074),mat(h,1.2),16,8)
        for j in range(5):
            x=-1.36+.68*j
            box('Muro del fondo',(x,1.68,.91),(.67,.14,.94),dark)
            box('Panel del fondo',(x,1.594,1.04),(.39,.025,.56),stone)
            label('JQFFT'[j],(x-.097,1.574,.98),.27,gilt,(pi/2,0,0),.012)
        for j in range(11):
            y=-.84+j*.22
            for side in (-1,1):
                for k in range(3):
                    x=side*(.47+k*.31)
                    box('Baldosa pulida',(x,y,plinth+.052),(.28,.197,.013),mat(['829183','687c70','9c9d79'][(j+k)%3],0,.22))
        for x,y in [(-2.05,-.52),(2.02,-.28),(-1.96,.72),(1.96,.93)]:
            z=height(x,y)
            beam('Obelisco exterior',(x,y,z),(x,y,z+.42),.16,dark,.079,4)
            ico('Remate de bronce',(x,y,z+.49),(.09,.09,.14),gilt,1)
        for side in (-1,1):
            road('Vía del palacio',[(side*(.48+.62*t),-1.46-.88*t) for t in (j/40 for j in range(41))],.035,gilt)
    else:
        cx,cy=-.18,.38
        for r in (.62,1.05,1.52):
            road('Red urbana',circle(cx,cy,r),.036,aqua)
        for k in range(8):
            a=k*tau/8
            road('Avenida sinaptica',[(cx+r*cos(a),cy+r*sin(a)) for r in (.16+j*.071 for j in range(25))],.022,mat('d1a4e9',.46))
        for k in range(34):
            a=k*2.399; r=.23*sqrt(k); x=cx+r*cos(a); y=cy+r*sin(a)
            z=height(x,y); h=(.29+.85*(1-r/1.43))*rnd.uniform(.69,1.25)
            tower(x,y,z,.08+rnd.random()*.055,h,mat(rnd.choice(['9883c3','b797ce','6d75ae']),.07,.28),aqua)
        uv('Nucleo de enlace',(cx,cy,1.45),(.22,.22,.21),mat('ffc2f5',1.7),28,14)
        for a in (0,pi/2):
            tube('Arcos de enlace',[(cx+.42*cos(t)*cos(a),cy+.42*cos(t)*sin(a),1.35+.41*sin(t)) for t in (k*pi/48 for k in range(49))],.025,mat('edadee',.65))
        for j in range(18):
            a=j*tau/18
            x=2.05*cos(a); y=2.05*sin(a); z=height(x,y)
            r=.12+.04*(j%3)
            beam('Distrito cristalino',(x,y,z),(x,y,z+.28+.09*(j%4)),r,mat(['a28bcb','8ba7cf','c2afd9'][j%3],.05,.20),r*.60,6)
            ico('Cúpula cristalina',(x,y,z+.32+.09*(j%4)),(r*.76,r*.76,r),mat('a7e4ed',.43),1)
        for x,y in [(-1.64,-1.14),(1.36,-1.03),(1.38,1.33)]:
            z=height(x,y)
            beam('Pabellón de conexión',(x,y,z),(x,y,z+.12),.32,city,.32,12)
            uv('Techo orgánico',(x,y,z+.12),(.31,.31,.18),mat('ccaed9',.18,.20),24,12)
        scatter(100,lambda x,y:(x-cx)**2+(y-cy)**2>2.86,lambda x,y:tree(x,y,rnd.uniform(.14,.32),['a25fb5','c083c1','794f9f']))

    habitantes.locals_on(g,i,height)

# Crea la cupula translucida del mundo
def atmosphere(g,i,edge,radius):
    tint=['db83b9','eba88c','b6d999','b9cba2','a398ee'][i]
    m=bpy.data.materials.new('Atmósfera translúcida | hemisferio superior')
    m.diffuse_color=(*g.rgb(tint),.17)
    m['flight_tint']=tint
    m.use_nodes=True
    nodes=m.node_tree.nodes; links=m.node_tree.links
    nodes.clear()
    layer=nodes.new('ShaderNodeLayerWeight')
    ramp=nodes.new('ShaderNodeMapRange')
    ramp.interpolation_type='SMOOTHERSTEP'
    ramp.inputs['From Min'].default_value=.63
    ramp.inputs['To Min'].default_value=.062
    ramp.inputs['To Max'].default_value=.66
    links.new(layer.outputs['Facing'],ramp.inputs['Value'])
    tint_shader=nodes.new('ShaderNodeEmission')
    tint_shader.inputs[0].default_value=(*g.rgb(tint),1)
    tint_shader.inputs[1].default_value=.67
    mix=nodes.new('ShaderNodeMixShader')
    links.new(ramp.outputs[0],mix.inputs[0])
    links.new(nodes.new('ShaderNodeBsdfTransparent').outputs[0],mix.inputs[1])
    links.new(tint_shader.outputs[0],mix.inputs[2])
    links.new(mix.outputs[0],nodes.new('ShaderNodeOutputMaterial').inputs[0])
    # Transparencia mezclada: tine la vista sin refraccion ni mascara de profundidad
    m.surface_render_method='BLENDED'
    m.use_transparency_overlap=False
    seg=128; rings=48
    points=[]; faces=[]
    for j in range(rings):
        t=j*pi/(2*rings)
        for k in range(seg):
            a=k*tau/seg
            r=(radius+(edge(a)-radius)*cos(t)**3)*cos(t)
            points.append((r*cos(a),r*sin(a),radius*sin(t)))
    points.append((0,0,radius))
    for j in range(rings-1):
        for k in range(seg):
            a=j*seg+k; b=j*seg+(k+1)%seg
            faces.append((a,b,b+seg,a+seg))
    for k in range(seg):
        faces.append(((rings-1)*seg+k,(rings-1)*seg+(k+1)%seg,len(points)-1))
    obj=model('Atmósfera | mitad superior',points,faces,[m])
    for f in obj.data.polygons: f.use_smooth=True


# Crea el sol: fotosfera con ruido y protuberancias
def solar(g):
    photosphere=g.uv('Fotosfera | granulos y manchas',(0,0,0),(3,3,3),g.mat('ff9c35'),128,64)
    m=photosphere.data.materials[0]; ns=m.node_tree.nodes; links=m.node_tree.links
    ns.clear()
    tc=ns.new('ShaderNodeTexCoord')
    n=ns.new('ShaderNodeTexNoise'); n.inputs['Scale'].default_value=7.8; n.inputs['Detail'].default_value=5.5; n.inputs['Roughness'].default_value=.72
    links.new(tc.outputs['Generated'],n.inputs['Vector'])
    granules=ns.new('ShaderNodeTexVoronoi'); granules.feature='DISTANCE_TO_EDGE'; granules.inputs['Scale'].default_value=75
    links.new(tc.outputs['Generated'],granules.inputs['Vector'])
    add=ns.new('ShaderNodeMath'); add.operation='MULTIPLY_ADD'; add.inputs[1].default_value=.46
    links.new(granules.outputs[0],add.inputs[0]); links.new(n.outputs['Fac'],add.inputs[2])
    ramp=ns.new('ShaderNodeValToRGB')
    for j,(p,c) in enumerate([(0.27,'622518'),(.43,'d04b1b'),(.59,'ff8a26'),(.73,'ffc34d'),(.93,'fff2b0')]):
        e=ramp.color_ramp.elements[0 if j==0 else -1] if j in (0,4) else ramp.color_ramp.elements.new(p)
        e.position=p; e.color=(*g.rgb(c),1)
    links.new(add.outputs[0],ramp.inputs[0])
    e=ns.new('ShaderNodeEmission'); e.inputs[1].default_value=2.25
    links.new(ramp.outputs[0],e.inputs[0]); links.new(e.outputs[0],ns.new('ShaderNodeOutputMaterial').inputs[0])
    rnd=random.Random(813)
    for a in (.25,.53,1.91,2.81,4.29,5.48):
        for j in range(5):
            shift=j*.029; h=rnd.uniform(.15,.52); w=rnd.uniform(.08,.16)
            coords=[]
            for k in range(39):
                t=k*pi/38; u=a+shift+w*cos(t)
                r=2.96+h*sin(t)**1.14
                coords.append((r*cos(u),.34*sin(a*2)+.19*cos(t),r*sin(u)))
            g.tube('Protuberancia solar',coords,rnd.uniform(.008,.024),g.mat(rnd.choice(['ff7e29','ffae3d','ffce64']),2.3))

