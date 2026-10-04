//! ImageIO (macOS). HEIC er støttet fra macOS 10.13.
#![allow(unsafe_code)]

use std::ffi::c_void;

use objc2_core_foundation::{
    CFBoolean, CFData, CFDictionary, CFMutableData, CFNumber, CFRetained, CFString, CFType,
    CGPoint, CGRect, CGSize,
};
use objc2_core_graphics::{
    CGBitmapContextCreate, CGBitmapContextCreateImage, CGColorSpace, CGContext, CGImage,
    CGImageAlphaInfo,
};
use objc2_image_io::{
    kCGImagePropertyPixelHeight, kCGImagePropertyPixelWidth,
    kCGImageSourceCreateThumbnailFromImageAlways, kCGImageSourceCreateThumbnailWithTransform,
    kCGImageSourceThumbnailMaxPixelSize, CGImageDestination, CGImageSource,
};

use crate::{fit, HeicError, Rgb};

fn number(dict: &CFDictionary<CFString, CFType>, key: &CFString) -> Option<u32> {
    let v = dict.get(key)?;
    let n = v.downcast_ref::<CFNumber>()?;
    n.as_i64().and_then(|n| u32::try_from(n).ok())
}

pub fn decode(bytes: &[u8], max_side: u32) -> Result<Rgb, HeicError> {
    let data = CFData::from_bytes(bytes);
    // SAFETY: `data` er gyldig; ingen opsjoner.
    let source = unsafe { CGImageSource::with_data(&data, None) }
        .ok_or_else(|| HeicError::Corrupt("ImageIO kjente ikke igjen filen".into()))?;

    // SAFETY: indeks 0 finnes hvis kilden har minst ett bilde; ellers returneres None.
    let props = unsafe { source.properties_at_index(0, None) }
        .ok_or_else(|| HeicError::Corrupt("mangler bildeegenskaper".into()))?;
    // SAFETY: egenskapsordboken har CFString-nøkler og CFType-verdier.
    let props: CFRetained<CFDictionary<CFString, CFType>> =
        unsafe { CFRetained::cast_unchecked(props) };
    // SAFETY: statiske konstanter fra ImageIO.
    let (key_w, key_h) = unsafe { (kCGImagePropertyPixelWidth, kCGImagePropertyPixelHeight) };
    let full_width =
        number(&props, key_w).ok_or_else(|| HeicError::Corrupt("mangler bredde".into()))?;
    let full_height =
        number(&props, key_h).ok_or_else(|| HeicError::Corrupt("mangler høyde".into()))?;
    let (tw, th) = fit(full_width, full_height, max_side);

    let max_px = CFNumber::new_i32(tw.max(th) as i32);
    // SAFETY: statiske konstanter fra ImageIO.
    let keys: [&CFString; 3] = unsafe {
        [
            kCGImageSourceCreateThumbnailFromImageAlways,
            kCGImageSourceThumbnailMaxPixelSize,
            kCGImageSourceCreateThumbnailWithTransform,
        ]
    };
    let values: [&CFType; 3] = [CFBoolean::new(true), &max_px, CFBoolean::new(false)];
    let opts = CFDictionary::<CFString, CFType>::from_slices(&keys, &values);
    // SAFETY: opsjonene har riktige typer (CFBoolean, CFNumber).
    let image = unsafe { source.thumbnail_at_index(0, Some(opts.as_opaque())) }
        .ok_or_else(|| HeicError::Corrupt("kunne ikke lage bilde".into()))?;

    let (width, height) = (CGImage::width(Some(&image)), CGImage::height(Some(&image)));
    let mut rgbx = vec![0u8; width * height * 4];
    let space =
        CGColorSpace::new_device_rgb().ok_or_else(|| HeicError::Corrupt("fargerom".into()))?;
    // SAFETY: bufferen er width*height*4 byte og lever lenger enn konteksten.
    let ctx = unsafe {
        CGBitmapContextCreate(
            rgbx.as_mut_ptr() as *mut c_void,
            width,
            height,
            8,
            width * 4,
            Some(&space),
            CGImageAlphaInfo::NoneSkipLast.0,
        )
    }
    .ok_or_else(|| HeicError::Corrupt("bitmap-kontekst".into()))?;
    let rect = CGRect::new(
        CGPoint::new(0.0, 0.0),
        CGSize::new(width as f64, height as f64),
    );
    CGContext::draw_image(Some(&ctx), rect, Some(&image));
    drop(ctx);

    let pixels = rgbx
        .chunks_exact(4)
        .flat_map(|p| [p[0], p[1], p[2]])
        .collect();
    Ok(Rgb {
        width: width as u32,
        height: height as u32,
        pixels,
        full_width,
        full_height,
    })
}

pub fn encode(width: u32, height: u32, rgb: &[u8]) -> Option<Vec<u8>> {
    let (w, h) = (width as usize, height as usize);
    let mut rgbx: Vec<u8> = rgb
        .chunks_exact(3)
        .flat_map(|p| [p[0], p[1], p[2], 255])
        .collect();
    let space = CGColorSpace::new_device_rgb()?;
    // SAFETY: bufferen er w*h*4 byte og lever lenger enn konteksten.
    let ctx = unsafe {
        CGBitmapContextCreate(
            rgbx.as_mut_ptr() as *mut c_void,
            w,
            h,
            8,
            w * 4,
            Some(&space),
            CGImageAlphaInfo::NoneSkipLast.0,
        )
    }?;
    let image = CGBitmapContextCreateImage(Some(&ctx))?;
    let out = CFMutableData::new(None, 0)?;
    let uti = CFString::from_static_str("public.heic");
    // SAFETY: gyldig mål og type; ett bilde.
    let dest = unsafe { CGImageDestination::with_data(&out, &uti, 1, None) }?;
    // SAFETY: `image` er et gyldig CGImage.
    let ok = unsafe {
        dest.add_image(&image, None);
        dest.finalize()
    };
    ok.then(|| out.to_vec())
}
