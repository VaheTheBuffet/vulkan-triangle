pub const IDENTITY: [f32; 16] = [
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 1.0, 0.0,
    0.0, 0.0, 0.0, 1.0
];

pub fn rot_z(angle: f32) -> [f32; 16] {
    [
        angle.cos(), angle.sin(), 0.0, 0.0,
        angle.sin(), -angle.cos(), 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        0.0, 0.0, 0.0, 1.0,
    ]
}

pub fn rot_y(angle: f32) -> [f32; 16] {
    [
        angle.cos(), 0.0, angle.sin(), 0.0,
        0.0, 1.0, 0.0, 0.0,
        angle.sin(), 0.0, -angle.cos(), 0.0,
        0.0, 0.0, 0.0, 1.0,
    ]
}

pub fn rot_x(angle: f32) -> [f32; 16] {
    [
        1.0, 0.0, 0.0, 0.0,
        0.0, angle.cos(), angle.sin(), 0.0,
        0.0, angle.sin(), -angle.cos(), 0.0,
        0.0, 0.0, 0.0, 1.0,
    ]
}

pub fn get_view_mat(cam_pos: [f32;3], look_pos: [f32;3], up: [f32; 3]) -> [f32; 16] {
    let mut forward = [cam_pos[0] - look_pos[0], cam_pos[1] - look_pos[1], cam_pos[2] - look_pos[2]];
    normalize(&mut forward);
    let right = cross(up, forward);
    let up = cross(forward, right);
    [
        right[0], right[1], right[2], -dot(right, cam_pos),
        up[0], up[1], up[2], -dot(up, cam_pos),
        forward[0], forward[1], forward[2], -dot(forward, cam_pos),
        0.0, 0.0, 0.0, 1.0
    ]
}

pub fn get_proj_mat(v_fov: f32, aspect_ratio: f32, near: f32, far: f32) -> [f32; 16] {
    [
        1.0 / v_fov.tan(), 0.0, 0.0, 0.0,
        0.0, 1.0 / (v_fov.tan()*aspect_ratio), 0.0, 0.0,
        0.0, 0.0, -far/(far-near), -far*near/(far-near),
        0.0, 0.0, -1.0, 0.0
    ]
}

#[inline(always)]
fn dot(v1: [f32; 3], v2: [f32; 3]) -> f32 {
    v1[0] * v2[0] + v1[1] * v2[1] + v1[2] * v2[2]
}

#[inline(always)]
fn cross(v1: [f32; 3], v2: [f32; 3]) -> [f32; 3] {
    [v1[1]*v2[2] - v2[1]*v1[2], v1[2]*v2[0] - v2[2]*v1[0], v1[0]*v2[1] - v2[0]*v1[1]] 
}

#[inline(always)]
fn normalize(v: &mut [f32; 3]) {
    let inv_len = 1.0/dot(*v, *v).sqrt();
    v[0] *= inv_len;
    v[1] *= inv_len;
    v[2] *= inv_len;
}