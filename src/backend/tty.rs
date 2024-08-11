use std::path::PathBuf;

use libc::dev_t;
use smithay::backend::allocator::gbm::{GbmAllocator, GbmDevice};
use smithay::backend::allocator::Fourcc;
use smithay::backend::drm::compositor::DrmCompositor;
use smithay::backend::drm::{DrmDevice, DrmDeviceFd};
use smithay::backend::renderer::element::surface::WaylandSurfaceRenderElement;
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::backend::session::libseat::LibSeatSession;
use smithay::backend::session::Session;
use smithay::desktop::space::SpaceRenderElements;
use smithay::reexports::calloop::{LoopHandle, RegistrationToken};

use crate::LoopData;

const SUPPORTED_COLOR_FORMATS: &[Fourcc] = &[Fourcc::Argb8888, Fourcc::Abgr8888];

type GbmDrmCompositor =
    DrmCompositor<GbmAllocator<DrmDeviceFd>, GbmDevice<DrmDeviceFd>, (), DrmDeviceFd>;

struct OutputDevice {
    id: dev_t,
    path: PathBuf,
    token: RegistrationToken,
    drm: DrmDevice,
    gles: GlesRenderer,
    drm_compositor: GbmDrmCompositor,
}

pub struct Tty {
    session: LibSeatSession,
    primary_gpu_path: PathBuf,
    output_device: Option<OutputDevice>,
}

impl Tty {
    pub fn seat_name(&self) -> String {
        self.session.seat()
    }

    pub fn renderer(&mut self) -> &mut GlesRenderer {
        &mut self.output_device.as_mut().unwrap().gles
    }

    pub fn render(
        &mut self,
        twm: &mut crate::Twm,
        elements: &[SpaceRenderElements<GlesRenderer, WaylandSurfaceRenderElement<GlesRenderer>>],
    ) {
        todo!()
    }
}

impl Tty {
    pub fn new(event_loop: LoopHandle<LoopData>) -> Self {
        todo!()
    }

    pub fn init(&self, twm: &mut crate::Twm) {
        todo!()
    }
}
