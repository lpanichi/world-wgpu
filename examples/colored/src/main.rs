use geometry::tesselation::{NalgebraTriangle, divide_triangle};
use gui::gpu::pipelines::colored::vertex::into_colored_vertex;
use iced::{
    Element,
    Length::Fill,
    widget::{center, shader},
};
mod program;

fn get_base_triangle() -> Vec<NalgebraTriangle> {
    let z = 0.0;
    vec![[
        [1.0, -1.0, z].into(),
        [0.0, 1.0, z].into(),
        [-1.0, -1.0, z].into(),
    ]]
}

enum Message {}

struct Colored {
    program: program::Program,
}

impl Colored {
    fn update(&mut self, _message: Message) {}
    fn view(&self) -> Element<'_, Message> {
        let shader = shader(&self.program).width(Fill).height(Fill);
        center(shader).into()
    }
}

impl Default for Colored {
    fn default() -> Self {
        let triangles = get_base_triangle();
        let triangles = divide_triangle(triangles.get(0).unwrap());
        let triangles = into_colored_vertex(triangles);
        println!("{:#?}", triangles);

        Self {
            program: program::Program {
                triangles: triangles,
            },
        }
    }
}

fn main() -> iced::Result {
    iced::application(Colored::default, Colored::update, Colored::view).run()
}
