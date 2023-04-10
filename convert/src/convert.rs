use std::os::raw::c_int;

extern "C" {
    // seems libyuv uses reverse byte order compared with our view

    pub fn ARGBRotate(
        src_argb: *const u8,
        src_stride_argb: c_int,
        dst_argb: *mut u8,
        dst_stride_argb: c_int,
        width: c_int,
        height: c_int,
        mode: c_int,
    ) -> c_int;

    pub fn ARGBMirror(
        src_argb: *const u8,
        src_stride_argb: c_int,
        dst_argb: *mut u8,
        dst_stride_argb: c_int,
        width: c_int,
        height: c_int,
    ) -> c_int;

    pub fn ARGBToI420(
        src_bgra: *const u8,
        src_stride_bgra: c_int,
        dst_y: *mut u8,
        dst_stride_y: c_int,
        dst_u: *mut u8,
        dst_stride_u: c_int,
        dst_v: *mut u8,
        dst_stride_v: c_int,
        width: c_int,
        height: c_int,
    ) -> c_int;

    pub fn NV12ToI420(
        src_y: *const u8,
        src_stride_y: c_int,
        src_uv: *const u8,
        src_stride_uv: c_int,
        dst_y: *mut u8,
        dst_stride_y: c_int,
        dst_u: *mut u8,
        dst_stride_u: c_int,
        dst_v: *mut u8,
        dst_stride_v: c_int,
        width: c_int,
        height: c_int,
    ) -> c_int;

    // I420ToRGB24: RGB little endian (bgr in memory)
    // I420ToRaw: RGB big endian (rgb in memory) to RGBA.
    pub fn I420ToRAW(
        src_y: *const u8,
        src_stride_y: c_int,
        src_u: *const u8,
        src_stride_u: c_int,
        src_v: *const u8,
        src_stride_v: c_int,
        dst_rgba: *mut u8,
        dst_stride_raw: c_int,
        width: c_int,
        height: c_int,
    ) -> c_int;

    pub fn I420ToARGB(
        src_y: *const u8,
        src_stride_y: c_int,
        src_u: *const u8,
        src_stride_u: c_int,
        src_v: *const u8,
        src_stride_v: c_int,
        dst_rgba: *mut u8,
        dst_stride_rgba: c_int,
        width: c_int,
        height: c_int,
    ) -> c_int;
}

pub fn bgra_to_i420(width: usize, height: usize, src: &[u8],dst_y_p: *mut u8,dst_u_p:*mut u8,dst_v_p:*mut u8)  {
    let dst_stride_y = width;
    let dst_stride_u = (width + 1) >>1;
    let dst_stride_v = dst_stride_u;
    // let tmp = (width * height) >>2;
    // let mut dst_y = Vec::new();
    // dst_y.resize(width*height, 0);
    // let mut dst_u =  Vec::new();
    // dst_u.resize(tmp, 0);
    // let mut dst_v =  Vec::new();
    // dst_v.resize(tmp, 0);
    unsafe {
        ARGBToI420(
            src.as_ptr(),
            (src.len() / height) as _,
            dst_y_p,
            dst_stride_y as _,
            dst_u_p,
            dst_stride_u as _,
            dst_v_p,
            dst_stride_v as _,
            width as _,
            height as _,
        );
    }
}

pub fn bgra_to_i420_n(width: usize, height: usize, src: &[u8]) ->(Vec<u8>,Vec<u8>,Vec<u8>){
    let dst_stride_y = width;
    let dst_stride_u = (width + 1) >>1;
    let dst_stride_v = dst_stride_u;
    let tmp = (width * height) >>2;
    let mut dst_y = Vec::new();
    dst_y.resize(width*height, 0);
    let mut dst_u =  Vec::new();
    dst_u.resize(tmp, 0);
    let mut dst_v =  Vec::new();
    dst_v.resize(tmp, 0);
    let dst_y_p = dst_y[..].as_mut_ptr();
    let dst_u_p =  dst_u[..].as_mut_ptr();
    let dst_v_p =  dst_v[..].as_mut_ptr();
    unsafe {
        ARGBToI420(
            src.as_ptr(),
            (src.len() / height) as _,
            dst_y_p,
            dst_stride_y as _,
            dst_u_p,
            dst_stride_u as _,
            dst_v_p,
            dst_stride_v as _,
            width as _,
            height as _,
        );
    }
    return (dst_y,dst_u,dst_v)
}
