use resvg::{
    tiny_skia::Pixmap,
    usvg::{self, Options, Transform, TreeParsing},
    Tree,
};

pub fn riptar_svg(size: u32, hash: u32, is_color: bool) -> String {
    let l = hash % 360;
    let mut color = "#111111ff".to_string();

    if is_color {
        color = format!("hsl({l}, 35%, 30%)");
    }

    format!(
        "<svg
        xmlns=\"http://www.w3.org/2000/svg\"
        version=\"1.1\"
        viewBox=\"0 0 {size} {size}\"
        width=\"{size}\"
        height=\"{size}\"
    >
        <defs>
        <filter
            id=\"nnnoise-filter\"
            x=\"0%\"
            y=\"0%\"
            width=\"100%\"
            height=\"100%\"
            filterUnits=\"objectBoundingBox\"
            primitiveUnits=\"userSpaceOnUse\"
            colorInterpolationFilters=\"linearRGB\"
        >
            <feTurbulence
                type=\"fractalNoise\"
                baseFrequency=\"0.04\"
                numOctaves=\"2\"
                seed=\"{hash}\"
                stitchTiles=\"stitch\"
                x=\"0%\"
                y=\"0%\"
                width=\"100%\"
                height=\"100%\"
                result=\"turbulence\"
                ></feTurbulence>
                <feSpecularLighting
                surfaceScale=\"15\"
                specularConstant=\"0.75\"
                specularExponent=\"20\"
                lightingColor=\"#fff\"
                x=\"0%\"
                y=\"0%\"
                width=\"100%\"
                height=\"100%\"
                in=\"turbulence\"
                result=\"specularLighting\"
            >
            <feDistantLight azimuth=\"3\" elevation=\"100\"></feDistantLight>
            </feSpecularLighting>
        </filter>
        </defs>
        <rect width=\"{size}\" height=\"{size}\" fill=\"{color}\"></rect>
        <rect
            width=\"{size}\"
            height=\"{size}\"
            fill=\"#ffffff\"
            filter=\"url(#nnnoise-filter)\"
        ></rect>
    </svg>"
    )
}

pub fn render_svg(svg: &str, size: u32) -> Vec<u8> {
    let usvg_tree = usvg::Tree::from_str(svg, &Options::default()).unwrap();
    let tree = Tree::from_usvg(&usvg_tree);
    let mut pixmap = Pixmap::new(size, size).unwrap();
    tree.render(Transform::default(), &mut pixmap.as_mut());
    pixmap.encode_png().unwrap()
}

pub fn djb2(str: &str) -> u32 {
    str.chars()
        .map(|c| c.to_digit(16).unwrap_or(1))
        .fold(5381, |hash, c| ((hash << 5) + hash) + c)
}