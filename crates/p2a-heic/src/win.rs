//! WIC (Windows). HEIC krever «HEIF Image Extensions» (og HEVC-støtte) fra Microsoft Store;
//! mange maskiner har dem fra produsenten. Mangler de, returneres `Unsupported`.
#![allow(unsafe_code)]

use windows::core::GUID;
use windows::Win32::Graphics::Imaging::{
    CLSID_WICImagingFactory, GUID_WICPixelFormat24bppRGB, IWICImagingFactory,
    WICBitmapDitherTypeNone, WICBitmapInterpolationModeFant, WICBitmapPaletteTypeCustom,
    WICDecodeMetadataCacheOnDemand,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
};

use crate::{fit, HeicError, Rgb};

fn corrupt(e: windows::core::Error) -> HeicError {
    HeicError::Corrupt(e.message())
}

pub fn decode(bytes: &[u8], max_side: u32) -> Result<Rgb, HeicError> {
    // SAFETY: COM-kall med gyldige argumenter. CoInitializeEx kan kalles flere ganger per
    // tråd (S_FALSE); vi kaller aldri CoUninitialize, så COM lever like lenge som tråden.
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let factory: IWICImagingFactory =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)
                .map_err(corrupt)?;
        let stream = factory.CreateStream().map_err(corrupt)?;
        stream.InitializeFromMemory(bytes).map_err(corrupt)?;
        // Uten HEIF-utvidelsen finnes ingen dekoder for formatet.
        let decoder = factory
            .CreateDecoderFromStream(
                &stream,
                std::ptr::null::<GUID>(),
                WICDecodeMetadataCacheOnDemand,
            )
            .map_err(|_| HeicError::Unsupported)?;
        let frame = decoder.GetFrame(0).map_err(corrupt)?;
        let (mut fw, mut fh) = (0u32, 0u32);
        frame.GetSize(&mut fw, &mut fh).map_err(corrupt)?;
        let (tw, th) = fit(fw, fh, max_side);

        let scaler = factory.CreateBitmapScaler().map_err(corrupt)?;
        scaler
            .Initialize(&frame, tw, th, WICBitmapInterpolationModeFant)
            .map_err(corrupt)?;
        let converter = factory.CreateFormatConverter().map_err(corrupt)?;
        converter
            .Initialize(
                &scaler,
                &GUID_WICPixelFormat24bppRGB,
                WICBitmapDitherTypeNone,
                None,
                0.0,
                WICBitmapPaletteTypeCustom,
            )
            .map_err(corrupt)?;
        let stride = tw * 3;
        let mut pixels = vec![0u8; (stride * th) as usize];
        converter
            .CopyPixels(std::ptr::null(), stride, &mut pixels)
            .map_err(corrupt)?;
        Ok(Rgb {
            width: tw,
            height: th,
            pixels,
            full_width: fw,
            full_height: fh,
        })
    }
}
