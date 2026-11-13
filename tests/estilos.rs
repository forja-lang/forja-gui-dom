//! Contrato con la hoja de estilos: toda clase `fg-*` que emite el renderizador
//! tiene que existir en `estilos.css`.
//!
//! Evita el fallo silencioso clásico: agregás un nodo, sale con una clase nueva
//! y en pantalla queda sin estilo (y nadie se entera hasta verlo "raro").

use forja_gui_dom::{render, NivelAviso, Nodo, TipoEntrada, Tono, VarianteBoton};

fn css() -> String {
    std::fs::read_to_string(format!("{}/estilos.css", env!("CARGO_MANIFEST_DIR")))
        .expect("no se pudo leer estilos.css")
}

/// Clases `fg-*` presentes en un HTML (sin repetir).
fn clases(html: &str) -> Vec<String> {
    let mut salida: Vec<String> = Vec::new();
    for trozo in html.split("class=\"").skip(1) {
        let Some(fin) = trozo.find('"') else { continue };
        for clase in trozo[..fin].split_whitespace() {
            if !salida.iter().any(|c| c == clase) {
                salida.push(clase.to_string());
            }
        }
    }
    salida
}

/// Árbol que usa todas las variantes del modelo.
fn arbol_completo() -> Nodo {
    Nodo::Contenedor {
        clase: "fg-columna".into(),
        hijos: vec![
            Nodo::Navegacion {
                enlaces: vec![("/".into(), "Inicio".into())],
            },
            Nodo::Encabezado {
                nivel: 1,
                texto: "Panel".into(),
            },
            Nodo::Tarjeta {
                titulo: "Resumen".into(),
                hijos: vec![
                    Nodo::Aviso {
                        texto: "todo bien".into(),
                        nivel: NivelAviso::Informacion,
                    },
                    Nodo::Progreso {
                        valor: 1.0,
                        maximo: 2.0,
                        etiqueta: "avance".into(),
                    },
                    Nodo::Insignia {
                        texto: "beta".into(),
                        tono: Tono::Aviso,
                    },
                    Nodo::Variable("total".into()),
                ],
            },
            Nodo::GrupoCampos {
                leyenda: "Alta".into(),
                hijos: vec![
                    Nodo::Entrada {
                        nombre: "titulo".into(),
                        tipo: TipoEntrada::Texto,
                        valor: String::new(),
                        marcador: "comprar pan".into(),
                        requerido: true,
                    },
                    Nodo::Seleccion {
                        nombre: "prioridad".into(),
                        etiqueta: "Prioridad".into(),
                        opciones: vec![("alta".into(), "Alta".into())],
                        seleccionada: String::new(),
                    },
                    Nodo::AreaTexto {
                        nombre: "detalle".into(),
                        valor: String::new(),
                        marcador: String::new(),
                        filas: 3,
                    },
                    Nodo::Casilla {
                        nombre: "urgente".into(),
                        etiqueta: "Urgente".into(),
                        marcado: false,
                    },
                ],
            },
            Nodo::Boton {
                texto: "Guardar".into(),
                accion: "guardar".into(),
                variante: VarianteBoton::Primario,
            },
            Nodo::Boton {
                texto: "Cancelar".into(),
                accion: "cancelar".into(),
                variante: VarianteBoton::Secundario,
            },
            Nodo::Tabla {
                cabeceras: vec!["a".into()],
                filas: vec![vec![Nodo::Texto("1".into())]],
            },
            Nodo::Lista {
                items: vec![Nodo::Texto("x".into())],
                clase: String::new(),
            },
            Nodo::Imagen {
                src: "/a.png".into(),
                alt: "a".into(),
            },
            Nodo::Enlace {
                href: "/b".into(),
                texto: "b".into(),
            },
            Nodo::Parrafo("hola".into()),
            Nodo::Dialogo {
                id: "d".into(),
                titulo: "t".into(),
                hijos: vec![],
                abierto: false,
            },
            Nodo::Cargando {
                texto: "cargando".into(),
            },
            Nodo::Pestanas {
                variable: "pestana".into(),
                items: vec![(
                    "uno".into(),
                    "Uno".into(),
                    vec![Nodo::Parrafo("panel".into())],
                )],
            },
            Nodo::ListaSeleccion {
                variable: "vista".into(),
                etiqueta: "Vista".into(),
                por_defecto: "hoy".into(),
                opciones: vec![("hoy".into(), "Hoy".into())],
            },
            Nodo::Plegable {
                titulo: "Más".into(),
                hijos: vec![Nodo::Texto("dentro".into())],
                abierto: false,
            },
            Nodo::Separador,
            Nodo::Contenedor {
                clase: "fg-fila".into(),
                hijos: vec![],
            },
        ],
    }
}

#[test]
fn todas_las_clases_emitidas_estan_en_el_css() {
    let html = render(&arbol_completo());
    let hoja = css();
    let faltantes: Vec<String> = clases(&html)
        .into_iter()
        .filter(|clase| !hoja.contains(&format!(".{clase}")))
        .collect();

    assert!(
        faltantes.is_empty(),
        "clases emitidas sin estilo en estilos.css: {faltantes:?}\n\
         (agregá la regla en library/gui-dom/estilos.css)"
    );
}

#[test]
fn el_arbol_completo_no_deja_marcas_sin_resolver() {
    let html = render(&arbol_completo());
    assert!(
        !html.contains("{{"),
        "quedaron marcas de plantilla en el HTML"
    );
    assert!(!html.contains("<script>"), "hay script sin escapar");
}

#[test]
fn documento_incluye_todo_lo_necesario_para_accesibilidad() {
    let html = forja_gui_dom::documento("Panel", &arbol_completo(), Some("/publico/estilos.css"));
    assert!(html.starts_with("<!doctype html><html lang=\"es\">"));
    assert!(html.contains("<meta name=\"viewport\""));
    assert!(html.contains("<meta charset=\"utf-8\">"));
    assert!(html.contains("<title>Panel</title>"));
    // Los diálogos nativos traen foco y escape del navegador.
    assert!(html.contains("<dialog"));
    // Los avisos anuncian su gravedad.
    assert!(html.contains("role=\"note\""));
}
