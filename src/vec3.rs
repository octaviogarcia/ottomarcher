#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub struct Vec3 {
    pub e: [f64;3]
}

impl std::fmt::Display for Vec3 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Vec3{{x: {}, y: {}, z: {}}}", self.x(), self.y(), self.z())
    }
}

// Type aliases for vec3
pub type Point3 = Vec3;   // 3D point
pub type Color  = Vec3;   // RGB color
pub type UnitVec3 = Vec3; //Type alias just to document unit vectors

use crate::utils;
use utils::MyRandom;
use utils::{max,min,abs};

impl Vec3{
    pub const ZERO : Self = Self{ e: [0.,0.,0.]};
    pub fn new(x: f64,y: f64,z: f64) -> Self { Self{e: [x,y,z]} }
    pub fn new_unit(x: f64,y: f64,z: f64) -> Self { Self{e: [x,y,z]}.unit() }

    pub fn x(&self) -> f64{ self.e[0] }
    pub fn y(&self) -> f64{ self.e[1] }
    pub fn z(&self) -> f64{ self.e[2] }

    pub fn dot(&self,other: Self) -> f64{
        self.e[0]*other.e[0]+self.e[1]*other.e[1]+self.e[2]*other.e[2]
    }
    pub fn length_squared(&self) -> f64 {
        self.dot(*self)
    }
    pub fn length(&self) -> f64 {
        self.length_squared().sqrt()
    }
    pub fn unit(&self) -> Self {
        *self / self.length()
    }
    pub fn abs(&self) -> Self {
        Vec3::new(abs(self.x()),abs(self.y()),abs(self.z()))
    }
    pub fn max(&self,v: &Vec3) -> Self {
        Vec3::new(max(self.x(),v.x()),max(self.y(),v.y()),max(self.z(),v.z()))
    }
    pub fn min(&self,v: &Vec3) -> Self {
        Vec3::new(min(self.x(),v.x()),min(self.y(),v.y()),min(self.z(),v.z()))
    }
    pub fn cross(&self, other: Self) -> Self{
        Self{ e : [
            self.e[1]*other.e[2] - self.e[2]*other.e[1],
            self.e[2]*other.e[0] - self.e[0]*other.e[2],
            self.e[0]*other.e[1] - self.e[1]*other.e[0],
        ]}
    }
    pub fn near_zero(&self) -> bool {
        let eps = 1e-8;
        return (self.e[0].abs() < eps) && (self.e[1].abs() < eps) && (self.e[2].abs() < eps);
    }
}

//Random implementations
impl MyRandom for Vec3{
    fn rand() -> Self { Self{e: [f64::rand(),f64::rand(),f64::rand()]} }
    fn rand_range(fmin: f64,fmax: f64) -> Self {
        Self{e: [
            f64::rand_range(fmin,fmax),
            f64::rand_range(fmin,fmax),
            f64::rand_range(fmin,fmax),
        ]}
    }
}

impl Vec3{
    pub fn rand_in_unit_sphere() -> Self{
        loop {
            let p = Self::rand_range(-1.,1.);
            if p.length_squared() < 1. {return p;};
        }
    }
    pub fn rand_unit_vector() -> Self{
        return Self::rand_in_unit_sphere().unit();
    }
    pub fn rand_in_hemisphere(normal: &Self) -> Self{
        let in_unit_sphere = Self::rand_in_unit_sphere();
        if in_unit_sphere.dot(*normal) > 0. {//Same hemisphere
            return in_unit_sphere;
        }
        return -in_unit_sphere;//Invert it
    }
    pub fn rand_in_unit_disc() -> Self{
        loop {
            let p = Self::new(f64::rand_range(-1.,1.),f64::rand_range(-1.,1.),0.);
            if p.length_squared() < 1. {return p;};
        }
    }
}

use std::ops::{Neg,Index,IndexMut,AddAssign,SubAssign,MulAssign,DivAssign,Add,Sub,Mul,Div};

impl Neg for Vec3{
    type Output = Self;
    fn neg(self) -> Vec3{ Vec3{ e : [-self.e[0],-self.e[1],-self.e[2]]} }
}
impl Index<usize> for Vec3{
    type Output = f64;
    fn index(&self, idx: usize) -> &Self::Output  { &self.e[idx]  }
}
impl IndexMut<usize> for Vec3{
    fn index_mut(&mut self, idx: usize) -> &mut Self::Output { &mut self.e[idx]  }
}
impl AddAssign<Vec3> for Vec3{
    fn add_assign(&mut self, other: Vec3){
        *self = Vec3 { e: [
            self.e[0]+other.e[0],
            self.e[1]+other.e[1],
            self.e[2]+other.e[2],
        ]}
    }
}
impl SubAssign<Vec3> for Vec3{
    fn sub_assign(&mut self, other: Vec3){
        *self += -other;
    }
}
impl MulAssign<Vec3> for Vec3{
    fn mul_assign(&mut self, other: Vec3){
        *self = Vec3 { e: [
            self.e[0]*other.e[0],
            self.e[1]*other.e[1],
            self.e[2]*other.e[2],
        ]}
    }
}
impl MulAssign<f64> for Vec3{
    fn mul_assign(&mut self, scalar: f64){
        *self = Vec3 { e: [
            self.e[0]*scalar,
            self.e[1]*scalar,
            self.e[2]*scalar,
        ]}
    }
}
impl DivAssign<f64> for Vec3{
    fn div_assign(&mut self, scalar: f64){
        *self *= 1./scalar;
    }
}
impl Add<Vec3> for Vec3{
    type Output = Self;
    fn add(self, other: Vec3) -> Vec3{
        Vec3 { e : [
            self.e[0]+other.e[0],
            self.e[1]+other.e[1],
            self.e[2]+other.e[2],
        ]}
    }
}
impl Sub<Vec3> for Vec3{
    type Output = Self;
    fn sub(self, other: Vec3) -> Vec3{
        self + (-other)
    }
}
impl Mul<Vec3> for Vec3{
    type Output = Self;
    fn mul(self, other: Vec3) -> Vec3{
        Vec3 { e : [
            self.e[0]*other.e[0],
            self.e[1]*other.e[1],
            self.e[2]*other.e[2],
        ]}
    }
}
impl Div<Vec3> for Vec3{
    type Output = Self;
    fn div(self, other: Vec3) -> Vec3{
        Vec3 { e : [
            self.e[0]/other.e[0],
            self.e[1]/other.e[1],
            self.e[2]/other.e[2],
        ]}
    }
}
impl Mul<f64> for Vec3{
    type Output = Self;
    fn mul(self, scalar: f64) -> Vec3{
        Vec3 { e : [
            self.e[0]*scalar,
            self.e[1]*scalar,
            self.e[2]*scalar,
        ]}
    }
}
impl Mul<Vec3> for f64{
    type Output = Vec3;
    fn mul(self, vec3: Vec3) -> Vec3{
        return vec3*self;
    }
}
impl Div<f64> for Vec3{
    type Output = Self;
    fn div(self, scalar: f64) -> Vec3{
        self * (1./scalar)
    }
}