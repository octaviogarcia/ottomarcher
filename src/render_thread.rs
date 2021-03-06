use rand::prelude::*;
use crate::math::vec3::*;
use crate::hits::*;
use crate::camera::*;
use crate::ray::*;
use std::sync::atomic::{AtomicU64, Ordering};
use crate::utils::{lerp,MyRandom,normalize_color,BloomFilter};

#[derive(Copy,Clone)]
pub struct Stats{//https://en.wikipedia.org/wiki/Algorithms_for_calculating_variance#Welford's_online_algorithm
    pub n: u32,
    pub sum: Color,
    pub color: (u8,u8,u8),
    pub bad_avgs: u32,
    pub avg_depth: f32,
    pub bloom_filter: BloomFilter,
}
impl Stats{
    pub fn new() -> Self {
        Self{sum:Color::ZERO,n:0,color:(0,0,0),bad_avgs: 0,avg_depth: 0.,bloom_filter: BloomFilter::new()}
    }
    #[inline]
    pub fn add(&mut self,x: &Color,depth: f32,obj_id: u64) -> bool{
        let old_color = self.color;
        self.sum   += x;
        self.n     += 1;
        self.color = {            
            let avg    = self.sum / self.n as f32;
            normalize_color(&avg).to_u8x3()
        };
        let bad_run = (old_color.0 == self.color.0) 
                   && (old_color.1 == self.color.1) 
                   && (old_color.2 == self.color.2) ;// && self.n > 30;
        self.bad_avgs += bad_run as u32;
        self.bad_avgs *= bad_run as u32;
        self.avg_depth = ((self.n as f32-1.)*self.avg_depth+depth)/self.n as f32;
        self.bloom_filter.set(obj_id);
        return self.bad_avgs >= 5;
    }
}

#[derive(Copy,Clone)]
pub struct Pixel{
    pub c: Color,
    pub stats: Stats
}
impl Pixel{
    pub fn new() -> Self{
        Self{c: Color::ZERO,stats: Stats::new()}
    }
}

#[derive(Copy,Clone)]
pub struct PixelsBox {
    pub pixels: *mut Vec<Pixel>,
}
unsafe impl Send for PixelsBox{}

//This DOES NOT work... for some reason I need to struct init from main(), @CompilerBug ??
impl PixelsBox{
    #[allow(dead_code)]
    pub fn new(image_size: usize) -> Self{
        Self{pixels: &mut vec!(Pixel::new();image_size as usize)}
    }
}


struct ThreadPixels{
    //Assigned pixels to the threads
    pub list: Vec<usize>,
    pub len: usize,
    //When rendering, here are added pixels that need to be rendered next pass
    //After a pass is done, one is expected to swap() these 2 buffers
    pub backbuff_list: Vec<usize>,
    pub backbuff_len: usize,
}

impl ThreadPixels{
    pub fn new(expected_max_size: usize) -> Self{
        Self{list: Vec::with_capacity(expected_max_size),len: 0,
             backbuff_list: Vec::with_capacity(expected_max_size),backbuff_len: 0,}
    }
    pub fn push(&mut self,pxl_idx: usize){
        self.list.push(pxl_idx);
        self.len += 1;
        self.backbuff_list.push(9999);//Add garbage just to expand if len overflows expected_max_size
    }
    pub fn shrink_to_fit(&mut self){
        self.list.shrink_to_fit();
        self.backbuff_list.shrink_to_fit();
    }
    pub fn swap_buffers(&mut self){
        ::std::mem::swap(&mut self.list,&mut self.backbuff_list);
        self.len = self.backbuff_len;
        self.backbuff_len = 0;
    }
    pub fn add_run(&mut self,i: usize,done: bool){
        //If the pixel render is not done, keep adding to the back buffer to draw in the next iteration
        self.backbuff_list[self.backbuff_len] = self.list[i];
        self.backbuff_len+=1 - done as usize;
    }
}


#[inline]
fn handle_hit(hit_record: &mut Option<HitRecord>,curr_ray: &mut Ray,curr_color: &mut Color) -> (f32,u64){
    let depth: f32;
    let obj_id: u64;
    match hit_record {
        Some(hr) => {
            let rslt = hr.material.scatter(&curr_ray,&hr);
            *curr_color *= rslt.attenuation;
            *curr_ray = rslt.ray;
            depth = hr.t;
            obj_id = hr.obj_id;
        },
        None => {
            let t: f32 = 0.5*(curr_ray.dir.y() + 1.0);//r.dir.y() + 1.0);
            let lerped_sky_color = lerp(t,Color::new(1.0,1.0,1.0),Color::new(0.5,0.7,1.0));
            *curr_color *= lerped_sky_color;
            depth = f32::INFINITY;
            obj_id = 0;
        }
    }
    return (depth,obj_id);
}

fn ray_color(r: &Ray,world: &FrozenHittableList, depth: u32,tmin: f32,tmax: f32,u: f32,v: f32) -> (Color,f32,u64){
    let mut curr_color = Color::new(1.,1.,1.);
    let mut curr_ray: Ray = *r;
    //Get the depth with the first hit
    let (depthf,obj_id) = handle_hit(&mut world.first_hit(&curr_ray,tmin,tmax,u,v),&mut curr_ray,&mut curr_color);
    if depthf.is_infinite(){
        return (curr_color,f32::INFINITY,0);
    }
    for _i in 1..depth{
        let h = handle_hit(&mut world.hit(&curr_ray,tmin,tmax),&mut curr_ray,&mut curr_color);
        if h.0.is_infinite(){
            return (curr_color,depthf,obj_id);//Return the depth & ID of the first hit!!!!
        }
    }
    return (-Color::ZERO,depthf,obj_id);//If we run out of depth return -black
}

pub fn render(camera: &Camera,world: &FrozenHittableList,max_depth: u32,tmin: f32,tmax: f32,
    samples_per_pixel: u32,image_width: u32,image_height: u32,
    pixels_box: PixelsBox,tid: u32,assigned_thread: &Vec<u32>,samples_atom: &AtomicU64)
{
    //println!("len is {}",unsafe{&*pixels_box.pixels}.len());
    let image_width_f  = image_width as f32;
    let image_height_f = image_height as f32;
    let image_size = (image_width*image_height) as usize;

    let mut thread_pixels = ThreadPixels::new(image_size);
    for pos in 0..image_size {
        if assigned_thread[pos] == tid {
            thread_pixels.push(pos);
        }
    }
    thread_pixels.shrink_to_fit();

    //Construct a blue noise wannabe with a low discrepancy random number
    //https://en.wikipedia.org/wiki/Low-discrepancy_sequence#Construction_of_low-discrepancy_sequences
    let jitters: Vec<(f32,f32)> = {
        let mut ret: Vec<(f32,f32)> = Vec::with_capacity(samples_per_pixel as usize);
        for s in 0..samples_per_pixel{
            let sdiv = s / 2;
            let jitteri = (sdiv&1) as f32;//mod 2
            let jitterj = (s&1) as f32;//mod 2
            ret.push((jitteri,jitterj));
        }//produces (0,0),(0,1),(1,0),(1,1),(0,0),...
        ret.shuffle(&mut rand::thread_rng());
        ret
    };

    for _sample in 0..samples_per_pixel{
        //idx is a double indirection... indexes[pos_idx] is the pixel index in the main memory buffer
        for idx in 0..thread_pixels.len{
            let pxl_idx = thread_pixels.list[idx];
            let pixel: &mut Pixel = &mut unsafe{&mut *pixels_box.pixels}[pxl_idx];
            //Should never happen since we upkeep undone pixels with a backbuffer
            //assert!(curr_samples < samples_per_pixel);
            let line = (pxl_idx as u32) / image_width;
            let col  = (pxl_idx as u32) - image_width*line;
            let j_f = line as f32;
            let i_f = col as f32;

            let i_rand = (f32::rand() + jitters[pixel.stats.n as usize].0)/2.;
            let j_rand = (f32::rand() + jitters[pixel.stats.n as usize].1)/2.;
            let u = (i_f+i_rand)/(image_width_f-1.);
            let v = 1.0 - (j_f+j_rand)/(image_height_f-1.);
            let ray = camera.get_ray(u,v);
            let (pixel_color,depth,obj_id) = ray_color(&ray,&world,max_depth,tmin,tmax,u,v);
            let done = pixel.stats.add(&pixel_color,depth,obj_id);
            thread_pixels.add_run(idx,done);
            let log_samples = (done as u32)*(samples_per_pixel-pixel.stats.n) + 1;//+1 cause is done post increment
            //Inform left over samples or 1
            samples_atom.fetch_add(log_samples as u64,Ordering::Relaxed);
        }
        thread_pixels.swap_buffers();
    }
}