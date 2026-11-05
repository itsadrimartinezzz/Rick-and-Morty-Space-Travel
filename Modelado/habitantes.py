import bpy
from math import cos, sin, sqrt, tau
from mathutils import Vector


# Figura hecha de piezas agrupadas en una coleccion
class Figure:
    # Prepara la coleccion el origen y la escala de la figura
    def __init__(self, g, name, origin, scale):
        self.g=g
        self.origin=Vector(origin)
        self.scale=scale
        self.collection=bpy.data.collections.new(name)
        bpy.context.scene.collection.children.link(self.collection)

    # Pasa un punto local de la figura al mundo
    def point(self, xyz):
        return self.origin+self.scale*Vector(xyz)

    # Mueve una pieza a la coleccion de la figura
    def organize(self,obj):
        for collection in list(obj.users_collection):
            collection.objects.unlink(obj)
        self.collection.objects.link(obj)
        return obj

    # Crea una pieza ovalada
    def oval(self,name,p,axes,color):
        obj=self.g.uv(name,self.point(p),tuple(v*self.scale for v in axes),self.g.mat(color),16,8)
        return self.organize(obj)

    # Crea una extremidad entre dos puntos
    def limb(self,name,a,b,r,color,tip=None):
        obj=self.g.beam(name,self.point(a),self.point(b),r*self.scale,self.g.mat(color),
                        None if tip is None else tip*self.scale,9)
        return self.organize(obj)

    # Crea un trazo que sigue una lista de puntos
    def stroke(self,name,points,r,color):
        return self.organize(self.g.tube(name,[self.point(p) for p in points],r*self.scale,self.g.mat(color)))

    # Crea un par de ojos
    def eyes(self,x,y,z,separation=.15,size=.14,pupil='252431',white='fff8ea'):
        for side in (-1,1):
            x0=x+side*separation
            self.oval('Ojo',(x0,y,z),(size,size*.38,size),white)
            self.oval('Pupila',(x0+.024,y-size*.34,z+.006),(size*.31,size*.18,size*.34),pupil)


# Modela a Rick y Morty en la cabina
def crew(g):
    for rick,x in [(True,.39),(False,-.39)]:
        f=Figure(g,'Rick Sánchez' if rick else 'Morty Smith',(x,-.082,.39),.45)
        skin='e7c0a4' if rick else 'f1bc86'
        clothes='f5f2e7' if rick else 'ffd648'
        head_z=.99 if rick else .80
        f.oval('Torso',(0,0,.31),(.32,.20,.37),clothes)
        if rick:
            f.oval('Camisa celeste',(0,-.174,.33),(.126,.043,.28),'89d7db')
            for sign in (-1,1):
                f.stroke('Solapa',[(sign*.21,-.17,.58),(sign*.16,-.219,.40),(sign*.077,-.214,.32)],.018,'c4d4d4')
            f.oval('Base del cabello',(0,.072,head_z+.04),(.32,.23,.37),'89cfe7')
            for j in range(11):
                a=j*tau/11
                if sin(a)<-.78: continue
                f.limb('Punta del cabello',(cos(a)*.24,.074,head_z+.25*sin(a)),
                       (cos(a)*.47,.08,head_z+.46*sin(a)),.118,'9edcf0',.006)
        else:
            f.oval('Cabello castaño',(0,.08,head_z+.052),(.326,.266,.33),'865535')
        crew_face(f,rick,head_z,skin)
        for sign in (-1,1):
            f.oval('Oreja',(sign*.287,-.055,head_z-.028),(.067,.07,.093),skin)
            elbow=(sign*.30,-.23,.245)
            hand=(sign*.23,-.63,.31) if rick else (sign*.13,-.35,.19)
            f.limb('Manga',(sign*.24,0,.50),elbow,.084,clothes)
            f.limb('Antebrazo',elbow,hand,.064,clothes if rick else skin)
            f.oval('Mano',hand,(.080,.069,.067),skin)
        if rick:
            f.stroke('Volante',[(.30*cos(a),-.66,.31+.20*sin(a)) for a in (k*tau/36 for k in range(37))],.031,'35484a')


# Dibuja la cara de un tripulante
def crew_face(f,rick,z0,skin):
    height=.33 if rick else .27
    head=f.g.uv('Cara',f.point((0,-.071,z0)),
                tuple(f.scale*r for r in (.286,.244,height)),f.g.mat(skin),40,24)
    f.organize(head)

    # Punto sobre la superficie de la cabeza
    def surface(x,z,offset=.005):
        depth=.244*sqrt(max(.06,1-(x/.286)**2-(z/height)**2))
        return x,-.071-depth-offset,z0+z

    # Crea un parche pegado a la cabeza
    def patch(name,x,z,width,tall,color,offset,bulge=0,lid=False):
        segments=40
        vertices=[f.point(surface(x,z,offset+bulge))]
        for ring in (1/3,2/3,1):
            for k in range(segments):
                angle=tau*k/segments
                u,v=cos(angle),sin(angle)
                if lid and v>0:
                    v*=.42
                vertices.append(f.point(surface(x+width*ring*u,z+tall*ring*v,
                                                offset+bulge*(1-ring*ring))))
        faces=[(0,1+j,1+(j+1)%segments) for j in range(segments)]
        for ring in (0,1):
            for j in range(segments):
                a=1+ring*segments+j
                b=1+ring*segments+(j+1)%segments
                faces.append((a,a+segments,b+segments,b))
        mesh=bpy.data.meshes.new(name); mesh.from_pydata(vertices,[],faces); mesh.update()
        obj=bpy.data.objects.new(name,mesh)
        f.collection.objects.link(obj)
        mesh.materials.append(f.g.mat(color))
        for face in mesh.polygons: face.use_smooth=True

    # Crea una linea de tinta pegada a la cabeza
    def ink(name,points,r,color,offset=.008):
        f.stroke(name,[surface(x,z,offset) for x,z in points],r,color)

    for side in (-1,1):
        x=side*(.105 if rick else .107)
        z=.065 if rick else .052
        width=.100 if rick else .089
        tall=.114 if rick else .096
        patch('Contorno de ojo',x,z,width+.005,tall+.005,'685353',.006,lid=rick)
        patch('Esclerótica',x,z,width,tall,'fff8eb',.010,.018,lid=rick)
        patch('Pupila',x+.018,z-.009,.027,.030,'282433',.032)
        patch('Brillo de pupila',x+.010,z+.002,.006,.006,'fffbf3',.034)
        if rick:
            ink('Párpado cansado',[(x-width+.009,z+.012),(x-.047,z+.039),
                                  (x+.005,z+.047),(x+.058,z+.042),(x+width-.008,z+.018)],
                .0075,'876965',.024)
        else:
            ink('Ceja preocupada',[(side*.183,.146),(side*.139,.161),(side*.096,.185),
                                    (side*.069,.185)],.011,'624232')
    if rick:
        ink('Ceja celeste',[(-.224,.154),(-.154,.175),(-.084,.186),(-.014,.174),
                            (.065,.180),(.148,.161),(.211,.141)],.019,'85bfd6',.012)
        f.oval('Puente de nariz',(.002,-.309,z0-.027),(.039,.050,.074),skin)
        f.oval('Punta de nariz',(.027,-.349,z0-.067),(.050,.033,.034),skin)
        ink('Boca torcida',[(-.136,-.153),(-.083,-.174),(-.018,-.170),
                             (.056,-.156),(.113,-.157)],.0085,'5b393b')
        ink('Pliegue del mentón',[(-.027,-.215),(.018,-.217),(.055,-.205)],.006,'a87363')
        f.oval('Mancha verde',(.034,surface(.034,-.181,.012)[1],z0-.181),(.019,.012,.025),'89ad65')
    else:
        f.oval('Nariz pequeña',(.009,-.312,z0-.050),(.047,.046,.036),skin)
        ink('Boca preocupada',[(-.081,-.147),(-.042,-.121),(.002,-.108),
                                (.045,-.123),(.077,-.146)],.009,'6f4240')


# Crea un cuerpo humanoide basico
def humanoid(f,skin,shirt):
    f.oval('Torso',(0,0,.47),(.24,.15,.29),shirt)
    f.oval('Cabeza',(0,-.014,.85),(.22,.18,.22),skin)
    f.eyes(0,-.165,.90,.098,.074)
    f.stroke('Boca',[(-.073,-.185,.787),(0,-.201,.769),(.069,-.185,.787)],.011,'633a51')
    for side in (-1,1):
        f.limb('Brazo',(side*.17,0,.60),(side*.29,-.04,.35),.055,skin)
        f.limb('Pierna',(side*.098,.02,.28),(side*.125,-.009,.063),.067,shirt)
        f.oval('Bota',(side*.125,-.052,.068),(.083,.135,.058),'494258')


# Coloca a los habitantes propios de cada mundo
def locals_on(g,i,height):
    # Crea una figura vacia sobre el terreno
    def person(name,x,y,scale,z=None):
        return Figure(g,name,(x,y,height(x,y) if z is None else z),scale)
    if i==0:
        for j,(x,y,z) in enumerate([(-.60,-.39,.94),(.71,-.27,.94),(-1.38,-.57,.94),(.52,-2.02,None)]):
            f=person('Brosephamon' if j%2==0 else 'Visitante del resort',x,y,.31,z)
            if j%2==0:
                humanoid(f,'e5a3b1','8ecee6')
                for side in (-1,1):
                    tip=(side*.17,.012,1.21)
                    f.limb('Antena',(side*.10,.012,1.00),tip,.027,'e5a3b1')
                    f.oval('Mechón',tip,(.072,.065,.059),'8ecee6')
            else:
                humanoid(f,'e696b4','e9be6f')
    elif i==1:
        for j,(x,y) in enumerate([(-.17,-1.54),(-1.01,-.60),(1.43,-.61)]):
            f=person('Birdperson' if j==0 else 'Habitante alado',x,y,.66 if j==0 else .43)
            humanoid(f,'e3aa7d','9d4e4d')
            f.oval('Casco verde',(0,.03,.986),(.23,.19,.13),'82944b')
            f.limb('Cresta',(0,.035,.99),(0,.105,1.23),.082,'b9b764',.008)
            for side in (-1,1):
                for k in range(5):
                    f.limb('Pluma',(side*(.13+.06*k),.115,.68+.043*k),
                           (side*(.34+.116*k),.145,.26+.102*k),.071,'a29280' if k%2 else 'd2c2a3',.018)
    elif i==2:
        f=person('Beebo',-.29,-1.57,.54)
        fur='68a9cb'; belly='bcdfda'; teal='438f9f'
        f.oval('Lomo de Beebo',(0,.24,.43),(.46,.53,.36),fur)
        f.oval('Vientre',(0,.19,.28),(.397,.43,.225),belly)
        f.oval('Cabeza peluda',(0,-.19,.55),(.40,.35,.35),fur)
        f.oval('Hocico',(0,-.481,.392),(.26,.15,.124),belly)
        f.oval('Nariz',(0,-.62,.416),(.10,.032,.058),'817086')
        f.eyes(0,-.49,.605,.167,.137,'6e3c65','eca5cb')
        for sign in (-1,1):
            for k in range(3):
                f.oval('Pata',(sign*.31,-.19+k*.31,.14),(.142,.142,.139),fur)
                f.oval('Mancha',(sign*.33,.009+k*.21,.604),(.091,.108,.055),teal)
            f.limb('Mechón lateral',(sign*.24,-.20,.74),(sign*.40,-.11,.88),.088,fur,.009)
        f.stroke('Cola',[(0,.65,.40),(.17,.89,.45),(.44,1.04,.43),(.62,1.14,.48)],.086,fur)
        for k in range(3):
            f.oval('Banda rosada',(.24+k*.135,.94+k*.066,.45),(.067,.093,.087),'d499b5')
        for j,(x,y) in enumerate([(.52,-.77),(1.55,-1.14),(.23,.32)]):
            bird=person('Ave azul y amarilla',x,y,.25,height(x,y)+.35+j*.10)
            bird.oval('Cuerpo',(0,0,.34),(.19,.28,.16),'4f9ad5')
            bird.oval('Cabeza',(0,-.20,.43),(.125,.13,.135),'559dd8')
            bird.limb('Pico',(0,-.30,.40),(0,-.46,.36),.068,'f5ca5a',.005)
            for side in (-1,1):
                bird.limb('Ala',(side*.08,0,.34),(side*.49,.13,.45),.088,'7cc9e9',.012)
    elif i==3:
        for j,(x,y,letter,color) in enumerate([(-.75,-.73,'A','99d0ab'),(.79,-.64,'B','e0bc77'),(.72,.19,'R','a5cce2')]):
            f=person('Alphabetriano '+letter,x,y,.33,.49)
            from math import pi
            f.organize(g.label(letter,f.point((-.31,0,.04)),f.scale*.91,g.mat(color,.16),(pi/2,0,0),f.scale*.063))
            f.eyes(0,-.085,.52,.089,.072)
    else:
        for j,(x,y) in enumerate([(-.59,-1.45),(.31,-1.63),(1.31,-.64),(-1.53,-.42),(.94,-.98)]):
            f=person('Nippalian',x,y,.35 if j==0 else .27)
            humanoid(f,'6eaad6',['e8c168','b8a5df','7bbdb3'][j%3])
            for k in (-1,0,1):
                tip=(k*.14,.026,1.15+.04*(k==0))
                f.limb('Protuberancia',(k*.105,.026,1.00),tip,.031,'6eaad6')
                f.oval('Punta amarilla',tip,(.045,.044,.055),'f4d27b')
