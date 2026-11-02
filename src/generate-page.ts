import * as fs from "fs";
import * as fse from "fs-extra";
import * as path from "path";
import { Demopage } from "webpage-templates";

const data = {
    title: "Estereograma",
    description: "Generador de estereogramas (también conocidos como Magic Eye) que corre en la GPU con WebGL.",
    introduction: [
        "Un autoestereograma (también conocido como Magic Eye) es una imagen 2D hecha para crear la ilusión de 3D. En cada imagen hay un objeto 3D que solo se ve si mirás la imagen de cierta manera, como si la pantalla fuera transparente y estuvieras mirando la pared de atrás. Con la práctica se vuelve más fácil.",
        "Los autoestereogramas fueron muy populares en los años 90. Aprovechan la estereopsis: el cerebro reconstruye la profundidad combinando las dos imágenes levemente distintas que percibe cada ojo.",
        "En este proyecto podés usar tu propio mapa de profundidad, personalizar los patrones y la forma en que se calcula la imagen. Todo corre en la GPU y también puede mostrar escenas en movimiento en tiempo real."
    ],
    githubProjectName: "stereogram-webgl",
    readme: {
        filepath: path.join(__dirname, "..", "README.md"),
        branchName: "main"
    },
    additionalLinks: [
        { href: "https://piellardj.github.io/stereogram-solver/", text: "Mirá el solver de Magic Eye acá." },
    ],
    styleFiles: [],
    scriptFiles: [
        "script/gl-matrix-2.5.1-min.js",
        "script/main.min.js"
    ],
    indicators: [
        {
            id: "fps-indicator",
            label: "FPS"
        },
        {
            id: "stripes-count-indicator",
            label: "Franjas"
        },
        {
            id: "tilesize-indicator",
            label: "Tamaño del patrón"
        },
    ],
    canvas: {
        width: 512,
        height: 512,
        enableFullscreen: true
    },
    controlsSections: [
        {
            title: "Mapa de profundidad",
            controls: [
                {
                    type: Demopage.supportedControls.Checkbox,
                    title: "Mostrar escala de grises",
                    id: "show-heightmap-checkbox-id",
                    checked: false,
                },
                {
                    type: Demopage.supportedControls.Range,
                    title: "Profundidad",
                    id: "depth-range-id",
                    min: 0,
                    max: 1,
                    value: 1,
                    step: 0.01
                },
                {
                    type: Demopage.supportedControls.Checkbox,
                    title: "Invertir",
                    id: "invert-heightmap-checkbox-id",
                    checked: false,
                },
                {
                    type: Demopage.supportedControls.Tabs,
                    title: "Modo",
                    id: "heightmap-mode-tabs-id",
                    unique: true,
                    options: [
                        {
                            label: "Quieto",
                            value: "still",
                            checked: true,
                        },
                        {
                            label: "En movimiento",
                            value: "moving",
                        },
                    ]
                },
                {
                    type: Demopage.supportedControls.Select,
                    title: "Predefinido",
                    id: "heightmap-preset-select-id",
                    placeholder: "Personalizado",
                    options: [
                        {
                            value: "planet.png",
                            label: "Planeta",
                            checked: true,
                        },
                        {
                            value: "bumps.png",
                            label: "Relieves",
                        },
                        {
                            value: "smile.png",
                            label: "Sonrisa",
                        },
                        {
                            value: "hand.png",
                            label: "Mano",
                        },
                        {
                            value: "ripple.png",
                            label: "Onda",
                        },
                        {
                            value: "head.png",
                            label: "Cabeza",
                        },
                        {
                            value: "atomium.png",
                            label: "Atomium",
                        },
                        {
                            value: "car.png",
                            label: "Auto",
                        },
                        {
                            value: "dolphin.png",
                            label: "Delfín",
                        },
                        {
                            value: "tree.png",
                            label: "Árbol",
                        },
                        {
                            value: "ship.png",
                            label: "Barco",
                        }
                    ]
                },
                {
                    type: Demopage.supportedControls.Select,
                    title: "Predefinido",
                    id: "model-preset-select-id",
                    placeholder: "Personalizado",
                    options: [
                        {
                            value: "primitives",
                            label: "Primitivas",
                            checked: true,
                        },
                        {
                            value: "cube",
                            label: "Cubo",
                        },
                        {
                            value: "monkey",
                            label: "Mono",
                        },
                        {
                            value: "bunny",
                            label: "Conejo",
                        },
                        {
                            value: "atomium",
                            label: "Atomium",
                        }
                    ]
                },
                {
                    type: Demopage.supportedControls.FileUpload,
                    title: "Personalizado",
                    id: "input-model-upload-button",
                    accept: [".stl"],
                    defaultMessage: "Subir un STL"
                },
                {
                    type: Demopage.supportedControls.FileUpload,
                    title: "Personalizado",
                    id: "input-heightmap-upload-button",
                    accept: [".png", ".jpg", ".bmp", ".webp"],
                    defaultMessage: "Subir un mapa de profundidad"
                },
            ]
        },
        {
            title: "Patrón",
            controls: [
                {
                    type: Demopage.supportedControls.Tabs,
                    title: "Modo",
                    id: "tile-mode-tabs-id",
                    unique: true,
                    options: [
                        {
                            label: "Textura",
                            value: "texture",
                            checked: true,
                        },
                        {
                            label: "Ruido",
                            value: "noise"
                        },
                    ]
                },
                {
                    type: Demopage.supportedControls.Select,
                    title: "Predefinido",
                    id: "tile-preset-select-id",
                    placeholder: "Personalizado",
                    options: [
                        {
                            value: "wallpaper.png",
                            label: "Empapelado",
                        },
                        {
                            value: "space.png",
                            label: "Espacio",
                            checked: true,
                        },
                        {
                            value: "foliage.png",
                            label: "Follaje",
                        },
                        {
                            value: "giraffe.png",
                            label: "Jirafa",
                        },
                        {
                            value: "stones.png",
                            label: "Piedras",
                        },
                        {
                            value: "leaves.png",
                            label: "Hojas",
                        }
                    ]
                },
                {
                    type: Demopage.supportedControls.Range,
                    title: "Resolución",
                    id: "tile-noise-resolution-range-id",
                    min: 8,
                    max: 64,
                    value: 16,
                    step: 1
                },
                {
                    type: Demopage.supportedControls.Checkbox,
                    title: "Cuadrado",
                    id: "tile-noise-square-checkbox-id",
                    checked: true,
                },
                {
                    type: Demopage.supportedControls.Checkbox,
                    title: "Color",
                    id: "tile-noise-colored-checkbox-id",
                    checked: true,
                },
                {
                    type: Demopage.supportedControls.FileUpload,
                    title: "Personalizado",
                    id: "input-tile-upload-button",
                    accept: [".png", ".jpg", ".bmp", ".webp"],
                    defaultMessage: "Subir un patrón"
                },
            ]
        },
        {
            title: "Franjas",
            controls: [
                {
                    type: Demopage.supportedControls.Tabs,
                    title: "Franja principal",
                    id: "main-stripe-tabs-id",
                    unique: true,
                    options: [
                        {
                            value: "left",
                            label: "Izquierda",
                        },
                        {
                            value: "middle",
                            label: "Centro (beta)",
                            checked: true,
                        },
                    ]
                },
                {
                    type: Demopage.supportedControls.Tabs,
                    title: "Modo",
                    id: "stripes-mode-tabs-id",
                    unique: true,
                    options: [
                        {
                            value: "adaptative",
                            label: "Adaptativo",
                            checked: true,
                        },
                        {
                            value: "fixed",
                            label: "Fijo",
                        },
                    ]
                },
                {
                    type: Demopage.supportedControls.Range,
                    title: "Ancho de franja",
                    id: "stripes-width-range-id",
                    min: 20,
                    max: 200,
                    value: 80,
                    step: 1
                },
                {
                    type: Demopage.supportedControls.Range,
                    title: "Cantidad de franjas",
                    id: "stripes-count-range-id",
                    min: 8,
                    max: 24,
                    value: 16,
                    step: 1
                },
                {
                    type: Demopage.supportedControls.Checkbox,
                    title: "Mostrar UV",
                    id: "show-uv-checkbox-id",
                    checked: false,
                },
            ]
        },
        {
            title: "Salida",
            controls: [
                {
                    type: Demopage.supportedControls.Checkbox,
                    title: "Mostrar indicadores",
                    id: "show-indicators-checkbox-id",
                    checked: false,
                },
                {
                    type: Demopage.supportedControls.FileDownload,
                    id: "image-download-id",
                    label: "Descargar imagen"
                }
            ]
        }
    ]
};

const SRC_DIR = path.resolve(__dirname);
const DEST_DIR = path.resolve(__dirname, "..", "docs");
const minified = true;

const buildResult = Demopage.build(data, DEST_DIR, {
    debug: !minified,
});

// disable linting on this file because it is generated
buildResult.pageScriptDeclaration = "/* eslint-disable */\n" + buildResult.pageScriptDeclaration;

const SCRIPT_DECLARATION_FILEPATH = path.join(SRC_DIR, "ts", "page-interface-generated.ts");
fs.writeFileSync(SCRIPT_DECLARATION_FILEPATH, buildResult.pageScriptDeclaration);

fse.copySync(path.join(SRC_DIR, "static"), DEST_DIR);

const SITE_ORIGIN = "https://pablogventura.github.io/stereogram-webgl";
const REPO_URL = "https://github.com/pablogventura/stereogram-webgl";

function localizeGeneratedHtml(filepath: string): void {
    const replacements: Array<[string, string]> = [
        ["lang=\"en\"", "lang=\"es\""],
        ["https://github.com/piellardj/stereogram-webgl", REPO_URL],
        ["https://piellardj.github.io/stereogram-webgl", SITE_ORIGIN],
        ["More on GitHub", "Más en GitHub"],
        [">Explanations<", ">Explicaciones<"],
        [" - Explanations", " - Explicaciones"],
        ["You need to enable Javascript to run this experiment.", "Tenés que activar JavaScript para usar esta página."],
        ["title=\"Home\"", "title=\"Inicio\""],
        ["aria-label=\"Home\"", "aria-label=\"Inicio\""],
        ["title=\"Live project\"", "title=\"Ver en vivo\""],
        ["aria-label=\"Live project\"", "aria-label=\"Ver en vivo\""],
        ["Readme page of my project 'Estereograma'.", "Página de explicaciones del proyecto 'Estereograma'."],
    ];

    let html = fs.readFileSync(filepath, "utf8");
    for (const [from, to] of replacements) {
        html = html.split(from).join(to);
    }
    fs.writeFileSync(filepath, html);
}

localizeGeneratedHtml(path.join(DEST_DIR, "index.html"));
localizeGeneratedHtml(path.join(DEST_DIR, "readme", "index.html"));
