//! Imprime una página completa de ejemplo. La usa `scripts/probar-dom.sh` para
//! verificar con un DOM real (jsdom) que el HTML es válido y accesible.
use forja_gui_dom::{
    documento, NivelAviso, Nodo, TipoEntrada, Tono, VarianteBoton,
};

fn main() {
    let arbol = Nodo::Contenedor {
        clase: "fg-columna".into(),
        hijos: vec![
            Nodo::Navegacion {
                enlaces: vec![
                    ("/".into(), "Inicio".into()),
                    ("/tareas".into(), "Tareas".into()),
                ],
            },
            Nodo::Encabezado {
                nivel: 1,
                texto: "Tareas".into(),
            },
            Nodo::Aviso {
                texto: "Se guardó el cambio".into(),
                nivel: NivelAviso::Informacion,
            },
            Nodo::Tarjeta {
                titulo: "Hoy".into(),
                hijos: vec![
                    Nodo::Parrafo("2 pendientes".into()),
                    Nodo::Progreso {
                        valor: 1.0,
                        maximo: 2.0,
                        etiqueta: "avance de tareas".into(),
                    },
                    Nodo::Insignia {
                        texto: "en curso".into(),
                        tono: Tono::Aviso,
                    },
                ],
            },
            Nodo::GrupoCampos {
                leyenda: "Nueva tarea".into(),
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
                        opciones: vec![
                            ("alta".into(), "Alta".into()),
                            ("baja".into(), "Baja".into()),
                        ],
                        seleccionada: "baja".into(),
                    },
                    Nodo::AreaTexto {
                        nombre: "detalle".into(),
                        valor: String::new(),
                        marcador: "opcional".into(),
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
                texto: "Agregar".into(),
                accion: "agregar".into(),
                variante: VarianteBoton::Primario,
            },
            Nodo::Tabla {
                cabeceras: vec!["Tarea".into(), "Estado".into()],
                filas: vec![vec![
                    Nodo::Texto("comprar pan".into()),
                    Nodo::Insignia {
                        texto: "pendiente".into(),
                        tono: Tono::Neutro,
                    },
                ]],
            },
            Nodo::Dialogo {
                id: "confirmar".into(),
                titulo: "¿Borrar la tarea?".into(),
                hijos: vec![Nodo::Boton {
                    texto: "Borrar".into(),
                    accion: "borrar".into(),
                    variante: VarianteBoton::Primario,
                }],
                abierto: false,
            },
            Nodo::Imagen {
                src: "/publico/logo.svg".into(),
                alt: "Logo de Forja".into(),
            },
            Nodo::Cargando {
                texto: "Guardando".into(),
            },
        ],
    };
    print!("{}", documento("Tareas — Forja", &arbol, Some("/publico/estilos.css")));
}
