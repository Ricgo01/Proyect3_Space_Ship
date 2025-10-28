use raylib::prelude::*;

pub struct Framebuffer {
    pub width: u32,
    pub height: u32,
    pub color_buffer: Image,
    pub depth_buffer: Vec<f32>,
    background_color: Color,
    current_color: Color,
}

impl Framebuffer {
    pub fn new(width: u32, height: u32) -> Self {
        let color_buffer = Image::gen_image_color(width as i32, height as i32, Color::BLACK);
        Framebuffer {
            width,
            height,
            color_buffer,
            depth_buffer: vec![f32::INFINITY; (width * height) as usize],
            background_color: Color::BLACK,
            current_color: Color::WHITE,
        }
    }

    pub fn clear(&mut self) {
        self.color_buffer = Image::gen_image_color(self.width as i32, self.height as i32, self.background_color);
        self.depth_buffer.fill(f32::INFINITY);
    }

    pub fn set_pixel(&mut self, x: u32, y: u32) {
        if x < self.width && y < self.height {
            self.color_buffer.draw_pixel(x as i32, y as i32, self.current_color);
        }
    }

    pub fn set_background_color(&mut self, color: Color) {
        self.background_color = color;
    }

    pub fn set_current_color(&mut self, color: Color) {
        self.current_color = color;
    }

    pub fn set_pixel_color(&mut self, x: u32, y: u32, color: Color) {
        if x < self.width && y < self.height {
            self.color_buffer.draw_pixel(x as i32, y as i32, color);
        }
    }

    #[inline]
    pub fn depth_index(&self, x: u32, y: u32) -> usize { (y as usize) * (self.width as usize) + (x as usize) }

    pub fn test_and_set_depth(&mut self, x: u32, y: u32, depth: f32) -> bool {
        if x >= self.width || y >= self.height { return false; }
        let idx = self.depth_index(x, y);
        if depth < self.depth_buffer[idx] {
            self.depth_buffer[idx] = depth;
            true
        } else {
            false
        }
    }

    pub fn _render_to_file(&self, file_path: &str) {
        self.color_buffer.export_image(file_path);
    }

    pub fn swap_buffers(
        &self,
        window: &mut RaylibHandle,
        raylib_thread: &RaylibThread,
    ) {
        // we get the "new" data from the new buffer into texture
        if let Ok(texture) = window.load_texture_from_image(raylib_thread, &self.color_buffer) {

            // the window currently has the "old" data (previous frame)
            let mut renderer = window.begin_drawing(raylib_thread);

            // we move the "new" data to the window (current frame) 
            renderer.draw_texture(&texture, 0, 0, Color::WHITE);
        }
    }
}