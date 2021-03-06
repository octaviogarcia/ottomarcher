

use crate::math::vec3::{Vec3,UnitVec3,Point3};
use crate::math::vec4::{Vec4};
use crate::math::mat3x3::{Mat3x3};
use crate::math::mat4x4::{Mat4x4};
use crate::utils::{INF,get_id};
use crate::ray::Ray;
use crate::hits::HitRecord;
use crate::materials::Material;
use crate::bounding_box::*;

#[derive(Copy, Clone)]
pub struct Sphere {
    pub m_local_to_world: Mat4x4,
    pub m_word_to_local: Mat4x4,
    pub material: Material,
    bounding_box: BoundingBox,
}

impl Sphere {
    pub fn new(m_local_to_world: &Mat4x4,mat: &Material) -> Self{
        Sphere{m_local_to_world: *m_local_to_world,m_word_to_local: m_local_to_world.fast_homogenous_inverse(),material: *mat
              ,bounding_box: BoundingBox::draw_always()}
    }
    pub fn new_with_radius(o: &Point3,r: f32,mat: &Material) -> Self {
        let m = Mat4x4::new_translate(&o)
        .dot_mat(&Mat4x4::new_scale(&Vec3::new(r,r,r)));
        Sphere{m_local_to_world: m,m_word_to_local: m.fast_homogenous_inverse(),material: *mat
              ,bounding_box: BoundingBox::draw_always()}
    }
}

pub trait Traced: Bounded {
    fn hit(&self,r: &Ray,t_min: f32,t_max: f32) -> Option<HitRecord>;
}

impl Traced for Sphere {
    fn hit(&self,r: &Ray,t_min: f32,t_max: f32) -> Option<HitRecord> {
        let new_r = r.transform(&self.m_word_to_local);
        let a =  new_r.dir.length_squared();
        let half_b = new_r.orig.dot(new_r.dir);
        //Assume in local coords its a 1 radii sphere
        let c = new_r.orig.length_squared() - 1.;//self.radius*self.radius;
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
        let local_point = new_r.at(root);
        let point = self.m_local_to_world.dot_p3(&local_point);
        let outward_normal = self.m_local_to_world.dot_v3(&local_point).unit();
        //Maybe its faster to send some sort of reference/pointer to material? Probably not, since its so small
        return Some(HitRecord{t: root,point: point,normal: outward_normal,material: self.material,obj_id: get_id(self)});
    }
}

impl Bounded for Sphere {
    fn build_world_bounding_box(&self) -> BoundingBox3D {
        let ret = BoundingBox3D::new(&Point3::new(-1.,-1.,-1.),&Point3::new(1.,1.,1.)).dot(&self.m_local_to_world);
        //println!("{:?}",ret);
        return ret;
    }
    fn hit_bounding_box(&self,dir: &Vec3) -> bool{ 
        self.bounding_box.hit(dir) 
    }
    fn get_bounding_box(&self) -> BoundingBox { self.bounding_box }
}

#[derive(Copy, Clone)]
pub struct InfinitePlane {
    pub center: Point3,
    pub normal: UnitVec3,
    pub material: Material,
}

impl InfinitePlane {
    #[allow(dead_code)]
    pub fn new(center: &Point3,normal: &UnitVec3,material: &Material) -> Self{
        return InfinitePlane{center: *center,normal: normal.unit(),material: *material};
    }
}

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
    return *normal*((1. as f32).copysign(-normal_dot_dir));//if positive is same direction... reverse it
}

impl Traced for InfinitePlane {
    fn hit(&self,r: &Ray,t_min: f32,t_max: f32) -> Option<HitRecord> {
        let (root,normal_dot_dir) = ray_plane_intersect(r,&self.normal,&self.center);
        if root == INF || root < t_min || root > t_max {
            return None;
        }
        let outward_normal = normal_against_direction(&self.normal,normal_dot_dir);
        //Maybe its faster to send some sort of reference/pointer to material? Probably not, since its so small
        return Some(HitRecord{t: root,point: r.at(root),normal: outward_normal,material: self.material,obj_id: get_id(self)});
    }
}
impl Bounded for InfinitePlane {}

#[derive(Copy, Clone)]
pub struct Barycentric<const BT: usize>{
    pub origin: Point3,
    pub u: UnitVec3,
    pub u_length: f32,
    pub v: UnitVec3,
    pub v_length: f32,
    pub uxv: UnitVec3,
    pub uxvxu: UnitVec3,
    pub base_inv: Mat3x3,//Transforms a point (X,Y,Z) to the plane 2d coordinates
    pub v_in_base: Vec3,//Vec2, Z is 0 @SPEED
    pub material: Material,
    pub bounding_box: BoundingBox,
}

//Current rust version doesn't suport direct const enum templates... so I use an usize...
impl <const BT: usize> Barycentric<BT> {
    pub fn new(origin: &Point3,u: &UnitVec3,v: &UnitVec3,u_length: f32,v_length: f32,material: &Material) -> Self{
        let u_unit = u.unit();
        let v_unit = v.unit();
        let uxv = u_unit.cross(v_unit).unit();
        let uxvxu = uxv.cross(u_unit).unit();
        //Orthonormal basis, its inverse = transpose
        let base_inv = Mat3x3::new_3vec_vert(&u_unit,&uxv,&uxvxu).transpose();//.inverse();
        let v_in_base = base_inv.dot(&v_unit);
        return Self{origin: *origin,material: *material,
            u: u_unit,u_length: u_length,v: v_unit,v_length: v_length,uxv: uxv,uxvxu: uxvxu,
            base_inv: base_inv, v_in_base: v_in_base, bounding_box: BoundingBox::draw_always()
        };
    }
    pub fn new3points(origin: &Point3,upoint: &Point3,vpoint: &Point3,material: &Material) -> Self{
        let u_rel = *upoint-*origin;
        let u_length = u_rel.length();
        let v_rel = *vpoint-*origin;
        let v_length = v_rel.length();
        return Self::new(origin,&u_rel,&v_rel,u_length,v_length,material);
    }
    #[inline]
    fn calc_barycentric(&self,coords2d: &Point3) -> (f32,f32,f32){
        let rx = coords2d.x();
        let ry = coords2d.z();
        let ux = 1.;
        let uy = 0.;
        let vx = self.v_in_base.x();
        let vy = self.v_in_base.z();
        let det = ux*vy - vx*uy;
        let lambda1 = ( (rx*vy - vx*ry)/det)/self.u_length;
        let lambda2 = (-(rx*uy - ux*ry)/det)/self.v_length;
        return (lambda1,lambda2,1.-lambda1-lambda2);
    }
    #[inline]
    fn check_lambdas_triangle(&self,l1: f32,l2: f32,l3: f32) -> bool{
        return l1 > 0. && l2 > 0. && l3 > 0. && l1 < 1. && l2 < 1. && l3 < 1.;
    }
    #[inline]
    fn check_lambdas_parallelogram(&self,l1: f32,l2: f32,_l3: f32) -> bool{
        return l1 > 0. && l2 > 0. && l1 < 1. && l2 < 1.;
    }
    fn hit_aux(&self,r: &Ray,t_min: f32,t_max: f32) -> Option<HitRecord> {
        let (root,normal_dot_dir) = ray_plane_intersect(r,&self.uxv,&self.origin);
        if root == INF || root < t_min || root > t_max {
            return None;
        }
        let point = r.at(root);
        let point_from_origin = point - self.origin;
        let coords2d = self.base_inv.dot(&point_from_origin);//@SPEED: Y coord is useless, its 0 since we intersected
        let (lambda1, lambda2, lambda3) = self.calc_barycentric(&coords2d);
        let correct: bool;
        if BT == 0 {//Hope the compiler optimizes these static IFs!
            correct = self.check_lambdas_parallelogram(lambda1,lambda2,lambda3);
        }
        else if BT == 1 {
            correct = self.check_lambdas_triangle(lambda1,lambda2,lambda3);
        }
        else{
            panic!("Barycentric case not fulfilled");
        }
        if !correct { 
            return None;
        }
        let outward_normal = normal_against_direction(&self.uxv,normal_dot_dir);
        return Some(HitRecord{t: root,point: point,normal: outward_normal,material: self.material,obj_id: get_id(self)});
    }
}

impl <const BT: usize> Traced for Barycentric<BT> {
    fn hit(&self,r: &Ray,t_min: f32,t_max: f32) -> Option<HitRecord> {
        return self.hit_aux(r,t_min,t_max);
    }
}
impl <const BT: usize> Bounded for Barycentric<BT>{
    fn build_world_bounding_box(&self) -> BoundingBox3D {
        let a = self.u * self.u_length;
        let b = self.uxvxu * ((self.v*self.v_length).dot(self.uxvxu));
        let c = self.uxv;
        let m_local_to_world = Mat4x4::new_4vec_vert(
            &Vec4::new_v3(&a),&Vec4::new_v3(&b),&Vec4::new_v3(&c),&Vec4::new_p3(&self.origin)
        );
        let mul = 1. + (BT == 0) as u32 as f32;
        let end = Point3::new(mul,mul,0.1);//@HACK: this may be wrong... didn't really think about it
        return BoundingBox3D::new(&Point3::new(0.,0.,0.),&end).dot(&m_local_to_world);
    }
    fn hit_bounding_box(&self,dir: &Vec3) -> bool{ 
        self.bounding_box.hit(dir) 
    }
}

pub type Parallelogram = Barycentric<0>;
pub type Triangle = Barycentric<1>;


#[derive(Copy, Clone)]
pub struct Cube {
    pub m_local_to_world: Mat4x4,
    pub m_word_to_local: Mat4x4,
    pub material: Material,
    pub bounding_box: BoundingBox,
}
impl Cube {
    #[allow(dead_code)]
    pub fn new(m_local_to_world: &Mat4x4,mat: &Material) -> Self{
        Self{m_local_to_world: *m_local_to_world,m_word_to_local: m_local_to_world.fast_homogenous_inverse(),material: *mat,bounding_box: BoundingBox::draw_always()}
    }
    #[allow(dead_code)]
    pub fn new_with_length(o: &Point3,length: f32,mat: &Material) -> Self {
        let m = Mat4x4::new_translate(&o)
        .dot_mat(&Mat4x4::new_scale(&Vec3::new(length,length,length)));
        Self{m_local_to_world: m,m_word_to_local: m.fast_homogenous_inverse(),material: *mat,bounding_box: BoundingBox::draw_always()}
    }
}
/*
Intersection
{max(|x-c|,|y-c|,|z-c|) = r = ||p-c||_inf
{p = ut + o
=>
max(|ux t + ox - cx|,|uy t + oy - cy|,|uy t + oy - cz|) = r
=>
For each case
i = x,y,z
|ui t + oi - ci| = r
  ui t + oi - ci = r -> t1 =  ( r-oi+ci)/ui
-(ui t + oi - ci) = r -> t2 = (-r-oi+ci)/ui
- We only care when t > 0 (our ray always moves forward, "negative directions" are encoded in ui)
- plug t in in ||p-c||_inf = r. If it doesn't satisfy, it's not solution
=> We want the smallest positive t
*/

impl Traced for Cube {
    fn hit(&self,r: &Ray,t_min: f32,t_max: f32) -> Option<HitRecord> {
        let new_r = r.transform(&self.m_word_to_local);
        let mut smallest_t = INF;
        let mut idx = usize::MAX;
        for i in 0..3{
            if new_r.dir[i].abs() < 0.00001 {continue;}
            //In local coords r=0.5 and center=(0,0,0)
            let t1 = ( 0.5 - new_r.orig[i])/new_r.dir[i];
            let t2 = (-0.5 - new_r.orig[i])/new_r.dir[i];
            let t: f32;
            if t1 >= 0. && t2 >= 0.{
                t = t1.min(t2);
            }
            else {
                t = t1.max(t2);
            }
            if t > smallest_t || t > t_max || t < t_min {continue;}//We should check t < 0. but we already do that with t < t_min...
            let f = new_r.at(t).abs().max_val();
            let is_solution = (f-0.5).abs() <= 0.00001;
            if !is_solution {continue;}
            smallest_t = t;
            idx = i;
        }
        if idx == usize::MAX {return None;}
        let local_point = new_r.at(smallest_t);
        //Which argument in max(x,y,z) is "activated" defines the pair of faces
        //The sign of the inside (derivative of abs value) defines what face between the pair
        let local_outward_normal: Vec3 = [Vec3::new(1.,0.,0.),Vec3::new(0.,1.,0.),Vec3::new(0.,0.,1.)][idx]
                                  *(1. as f32).copysign(new_r.at(smallest_t)[idx]);
        let point = self.m_local_to_world.dot_p3(&local_point);
        let outward_normal = self.m_local_to_world.dot_v3(&local_outward_normal);
        return Some(HitRecord{t: smallest_t,point: point,normal: outward_normal,material: self.material,obj_id: get_id(self)});
    }
}

impl Bounded for Cube {
    fn build_world_bounding_box(&self) -> BoundingBox3D {
        return BoundingBox3D::new(&Point3::new(-1.,-1.,-1.),&Point3::new(1.,1.,1.)).dot(&self.m_local_to_world);
    }
    fn hit_bounding_box(&self,dir: &Vec3) -> bool{ 
        self.bounding_box.hit(dir) 
    }
    fn get_bounding_box(&self) -> BoundingBox { self.bounding_box }
}
