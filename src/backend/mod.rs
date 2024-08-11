use smithay::backend::renderer::element::surface::WaylandSurfaceRenderElement;
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::desktop::space::SpaceRenderElements;
use tty::Tty;
use winit::Winit;

use crate::Twm;

pub mod tty;
pub mod winit;

pub enum Backend {
    Tty(Tty),
    Winit(Winit),
}

impl Backend {
    pub fn init(&mut self, twm: &mut Twm) {
        match self {
            Backend::Tty(tty) => tty.init(twm),
            Backend::Winit(winit) => winit.init(twm),
        }
    }

    pub fn seat_name(&self) -> String {
        match self {
            Backend::Tty(tty) => tty.seat_name(),
            Backend::Winit(winit) => winit.seat_name(),
        }
    }

    pub fn renderer(&mut self) -> &mut GlesRenderer {
        match self {
            Backend::Tty(tty) => tty.renderer(),
            Backend::Winit(winit) => winit.renderer(),
        }
    }

    pub fn render(
        &mut self,
        twm: &mut Twm,
        elements: &[SpaceRenderElements<GlesRenderer, WaylandSurfaceRenderElement<GlesRenderer>>],
    ) {
        match self {
            Backend::Tty(tty) => tty.render(twm, elements),
            Backend::Winit(winit) => winit.render(twm, elements),
        }
    }

    pub fn tty(&mut self) -> &mut Tty {
        if let Self::Tty(v) = self {
            v
        } else {
            panic!("backend is not Tty");
        }
    }

    pub fn winit(&mut self) -> &mut Winit {
        if let Self::Winit(v) = self {
            v
        } else {
            panic!("backend is not Winit")
        }
    }
}
