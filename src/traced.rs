

use crate::vec3;
use vec3::{Vec3,UnitVec3,Point3};
use crate::utils::{INF};
use crate::ray::Ray;
use crate::hits::HitRecord;
use crate::materials::Material;

#[derive(Copy, Clone)]
pub struct Sphere {
    pub center: Point3,
    pub radius: f32,
    pub material: Material
}

pub trait Traced {
    fn hit(&self,r: &Ray,t_min: f32,t_max: f32) -> Option<HitRecord>;
}

impl Traced for Sphere {
    fn hit(&self,r: &Ray,t_min: f32,t_max: f32) -> Option<HitRecord> {
        let oc = r.orig - self.center;
        let a = r.dir.length_squared();
        let half_b = oc.dot(r.dir);
        let c = oc.length_squared() - self.radius*self.radius;
        let discriminant = half_b*half_b - a*c;
        if discriminant < 0. {
            return None;
        }
        let sqrtd = discriminant.sqrt();
        let mut root = (-half_b - sqrtd)/a;
        if root < t_min || root > t_max{
            root = (-half_b + sqrtd)/a;
            if root < t_min || root > t_max{
                return None;
            }
        }
        let point = r.at(root);
        let outward_normal = (point - self.center)/self.radius;
        //Maybe its faster to send some sort of reference/pointer to material? Probably not, since its so small
        return Some(HitRecord{t: root,point: point,normal: outward_normal,material: self.material});
    }
}

#[derive(Copy, Clone)]
pub struct InfinitePlane {
    pub center: Point3,
    pub normal: UnitVec3,
    pub material: Material,
}

impl InfinitePlane {
    pub fn new(center: &Point3,normal: &UnitVec3,material: &Material) -> Self{
        return InfinitePlane{center: *center,normal: normal.unit(),material: *material};
    }
}
/*
//Bunch of ifs to get expected values for the axis directions... probably a more consistent way...
if      normal.y() == 0. && normal.z() == 0.{//(nx 0 0) generates (0 nx 0),(0 0 nx)
    u = Vec3::new(0.,normal.x(),0.).unit();
    v = Vec3::new(0.,       0.,normal.x()).unit();
}
else if normal.x() == 0. && normal.z() == 0. {//(0 ny 0) generates (ny 0 0),(0 0 ny)
    u = Vec3::new(normal.y(),0.,0.).unit();
    v = Vec3::new(       0., 0.,normal.y()).unit();
}
else if normal.x() == 0. && normal.y() == 0. {//(0 0 nz) generates (nz 0 0),(0 nz 0)
    u = Vec3::new(normal.z(),       0.,0.).unit();
    v = Vec3::new(       0.,normal.z(),0.).unit();
}
else{
    u = Vec3::new(-normal.y(),normal.x(),0.).unit();
    v = normal.cross(u).unit();
}*/

#[inline]
fn ray_plane_intersect(r: &Ray,plane_normal: &UnitVec3,plane_center: &Point3) -> (f32,f32){
    let div = plane_normal.dot(r.dir);
    if div.abs() < 0.000001{//Plane is paralel to the ray
        return (INF,0.);
    }
    let num = -plane_normal.dot(r.orig - *plane_center);
    return (num/div,div);
}
#[inline]
fn normal_against_direction(normal: &Vec3,normal_dot_dir: f32) -> Vec3{
    return *normal*([1.,-1.][(normal_dot_dir > 0.) as usize]);//if positive is same direction... reverse it
}

impl Traced for InfinitePlane {
    fn hit(&self,r: &Ray,t_min: f32,t_max: f32) -> Option<HitRecord> {
        let (root,normal_dot_dir) = ray_plane_intersect(r,&self.normal,&self.center);
        if root == INF || root < t_min || root > t_max {
            return None;
        }
        let outward_normal = normal_against_direction(&self.normal,normal_dot_dir);
        //Maybe its faster to send some sort of reference/pointer to material? Probably not, since its so small
        return Some(HitRecord{t: root,point: r.at(root),normal: outward_normal,material: self.material});
    }
}

#[derive(Copy, Clone)]
pub struct Parallelogram {//Easier in parametric [f(t,s) = u*t+v*s+origin] form
    pub origin: Point3,
    pub normal: UnitVec3,
    pub u: UnitVec3,
    pub v: UnitVec3,
    pub u_length: f32,
    pub v_length: f32,
    pub material: Material,
}

impl Parallelogram {
    pub fn new(origin: &Point3,u: &UnitVec3,v: &UnitVec3,u_length: f32,v_length: f32,material: &Material) -> Self{
        //Rust doesn't have strict useful type aliases... so we re-unit u and v
        return Self{origin: *origin,normal: u.cross(*v).unit(),u: u.unit(),v: v.unit(),u_length: u_length,v_length: v_length,material: *material};
    }
    pub fn new3points(origin: &Point3,upoint: &Point3,vpoint: &Point3,material: &Material) -> Self{
        let u_rel = *upoint-*origin;
        let u_length = u_rel.length();
        let v_rel = *vpoint-*origin;
        let v_length = v_rel.length();
        return Self{origin: *origin,normal: u_rel.cross(v_rel).unit(),u: u_rel/u_length,v: v_rel/v_length,u_length: u_length,v_length: v_length,material: *material};
    }
}

/*
As the base is not orthonormal (its not a square), we can't just dot product, we need to correct due skewiness introduced by the other vector 
1) If we did just a dot product with u, in the zone "X" these points would get excluded because (x . u) > u_length
2) What we do instead is we obtain the vector "->" (named "corrected"), and then project that
1)      v_____   2)  v_____ 
        /..X./       /->X./
       / .../       / .../
      /  ../       /  ../
     /   ./       /   ./ 
    o____ u      o____ u  
*/
#[inline]
fn project_onto_vector(basis: &[&UnitVec3;2],point: &Point3,basis_idx: usize) -> f32{
    //assert basis_idx < basis.len()
    let onto  = *basis[basis_idx];
    let other = *basis[1 - basis_idx];
    let corrected = *point - point.dot(other)*other;
    //"onto" is unit length, and they are codirectional with "corrected" so it gets the length with sign
    //|uvector||u|cos t = |uvector|cos t
    return onto.dot(corrected);
}

impl Traced for Parallelogram {
    fn hit(&self,r: &Ray,t_min: f32,t_max: f32) -> Option<HitRecord> {
        let (root,normal_dot_dir) = ray_plane_intersect(r,&self.normal,&self.origin);
        if root == INF || root < t_min || root > t_max {
            return None;
        }
        let point = r.at(root);
        let point_from_origin = point - self.origin;
        let ucoord = project_onto_vector(&[&self.u,&self.v],&point_from_origin,0);
        if ucoord < 0. || ucoord > self.u_length{
            return None;
        }
        let vcoord = project_onto_vector(&[&self.u,&self.v],&point_from_origin,1);
        if vcoord < 0. || vcoord > self.v_length{
            return None;
        }
        let outward_normal = normal_against_direction(&self.normal,normal_dot_dir);
        return Some(HitRecord{t: root,point: point,normal: outward_normal,material: self.material});
    }
}


#[derive(Copy, Clone)]
pub struct Triangle {
    pub origin: Point3,
    pub normal: UnitVec3,
    pub u: UnitVec3,
    pub v: UnitVec3,
    pub w: UnitVec3,
    pub u_length: f32,
    pub v_length: f32,
    pub w_length: f32,
    pub material: Material,
}

impl Triangle {
    pub fn new(origin: &Point3,u: &UnitVec3,v: &UnitVec3,u_length: f32,v_length: f32,material: &Material) -> Self{
        //Rust doesn't have strict useful type aliases... so we re-unit u and v
        let u_un = u.unit();
        let v_un = v.unit();
        let w = (u_un*u_length) - (v_un*v_length);
        let w_length = w.length();
        return Self{origin: *origin,normal: u_un.cross(v_un).unit(),u: u_un,v: v_un,w: w/w_length,u_length: u_length,v_length: v_length,w_length: w_length,material: *material};
    }
    pub fn new3points(origin: &Point3,upoint: &Point3,vpoint: &Point3,material: &Material) -> Self{//@BUG SOMEWHERE!!!
        let u_rel = (*upoint)-(*origin);
        println!("u {}",u_rel);
        let u_length = u_rel.length();
        let v_rel = (*vpoint)-(*origin);
        println!("v {}",v_rel);
        let v_length = v_rel.length();
        let w_rel = u_rel - v_rel;
        let w_length = w_rel.length();
        return Self{origin: *origin,
            normal: u_rel.cross(v_rel).unit(),
            u: (u_rel/u_length),u_length: u_length,
            v: (v_rel/v_length),v_length: v_length,
            w: (w_rel/w_length),w_length: w_length,
            material: *material};
    }
}

impl Traced for Triangle {
    fn hit(&self,r: &Ray,t_min: f32,t_max: f32) -> Option<HitRecord> {
        let (root,normal_dot_dir) = ray_plane_intersect(r,&self.normal,&self.origin);
        if root == INF || root < t_min || root > t_max {
            return None;
        }
        let point = r.at(root);
        let point_from_origin = point - self.origin;
        //@SPEED: Triangles might have an easier and simpler implementation. I think you can just check with 2 multiplys, Havent though about it too much
        let ucoord = project_onto_vector(&[&self.u,&self.v],&point_from_origin,0)/self.u_length;
        if ucoord < 0. || ucoord > 1.{
            return None;
        }
        let vcoord = project_onto_vector(&[&self.u,&self.v],&point_from_origin,1)/self.v_length;
        if vcoord < 0. || vcoord > 1. || (ucoord+vcoord) > 1.{//Aditional check compared to Parallegram
            return None;
        }
        let outward_normal = normal_against_direction(&self.normal,normal_dot_dir);
        return Some(HitRecord{t: root,point: point,normal: outward_normal,material: self.material});
    }
}
/*
#[derive(Copy, Clone)]
pub struct Cube {
    pub center: Point3,
    pub normals: [UnitVec3;6],
    pub length: f32,
    pub material: Material
}

impl Cube{
    pub fn new(center: &Point3,length: f32,material: &Material) -> Self {
        let n = {}
        return Self{center: *center,length: length,material: *material};
    }
}

impl Traced for Cube {
    fn hit(&self,r: &Ray,t_min: f32,t_max: f32) -> Option<HitRecord> {
        let oc = r.orig - self.center;
        let a = r.dir.length_squared();
        let half_b = oc.dot(r.dir);
        let c = oc.length_squared() - self.radius*self.radius;
        let discriminant = half_b*half_b - a*c;
        if discriminant < 0. {
            return None;
        }
        let sqrtd = discriminant.sqrt();
        let mut root = (-half_b - sqrtd)/a;
        if root < t_min || root > t_max{
            root = (-half_b + sqrtd)/a;
            if root < t_min || root > t_max{
                return None;
            }
        }
        let point = r.at(root);
        let outward_normal = (point - self.center)/self.radius;
        //Maybe its faster to send some sort of reference/pointer to material? Probably not, since its so small
        return Some(HitRecord{t: root,point: point,normal: outward_normal,material: self.material});
    }
}*/