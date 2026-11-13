//! GUI sobre DOM: modelo de nodos + serializador HTML.
//!
//! Contexto: la GUI web de Forja dibuja en un `<canvas>`, lo que deja sin
//! accesibilidad, sin formularios nativos, sin CSS y sin SEO (B11 del plan web).
//! El camino elegido es **DOM isomórfico**: el mismo árbol de interfaz se
//! renderiza como elementos HTML reales en el navegador y como HTML en el
//! servidor (SSR).
//!
//! Este crate es el núcleo que se puede probar sin navegador:
//!
//! 1. [`Nodo`]: el árbol de interfaz (lo que produce el lenguaje).
//! 2. [`render`]: lo convierte en HTML con escape automático y atributos de
//!    accesibilidad (`aria-*`, `<label for>`, `role`).
//! 3. [`eventos`]: los atributos de eventos declarativos (`data-fg-onclick`)
//!    que el runtime del navegador engancha al DOM real.
//!
//! Lo que falta (siguiente paso): en `wasm-gui`, traducir `Layout` a [`Nodo`],
//! crear elementos reales con `web-sys` usando el mismo árbol y manejar los
//! eventos en vez de hacer hit-testing sobre rectángulos.

use std::fmt::Write as _;

/// Nodo de interfaz. Es el árbol que comparten servidor y navegador.
#[derive(Debug, Clone, PartialEq)]
pub enum Nodo {
    /// Contenedor genérico (`div`), con hijos.
    Contenedor { hijos: Vec<Nodo>, clase: String },
    /// Texto plano: se escapa siempre.
    Texto(String),
    /// Valor de una variable de la interfaz (binding de solo lectura).
    Variable(String),
    /// Encabezado de nivel 1..=6.
    Encabezado { nivel: u8, texto: String },
    /// Párrafo.
    Parrafo(String),
    /// Enlace.
    Enlace { href: String, texto: String },
    /// Botón con acción declarativa (`data-fg-accion`).
    Boton {
        texto: String,
        accion: String,
        variante: VarianteBoton,
    },
    /// Campo de texto con etiqueta accesible.
    Entrada {
        nombre: String,
        tipo: TipoEntrada,
        valor: String,
        marcador: String,
        requerido: bool,
    },
    /// Casilla con etiqueta.
    Casilla {
        nombre: String,
        etiqueta: String,
        marcado: bool,
    },
    /// Lista.
    Lista { items: Vec<Nodo>, clase: String },
    /// Tabla con cabeceras y filas.
    Tabla {
        cabeceras: Vec<String>,
        filas: Vec<Vec<Nodo>>,
    },
    /// Imagen con texto alternativo obligatorio (accesibilidad).
    Imagen { src: String, alt: String },
    /// Separador.
    Separador,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarianteBoton {
    Primario,
    Secundario,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TipoEntrada {
    Texto,
    Email,
    Contrasena,
    Numero,
    Busqueda,
    Fecha,
}

impl TipoEntrada {
    fn html(self) -> &'static str {
        match self {
            TipoEntrada::Texto => "text",
            TipoEntrada::Email => "email",
            TipoEntrada::Contrasena => "password",
            TipoEntrada::Numero => "number",
            TipoEntrada::Busqueda => "search",
            TipoEntrada::Fecha => "date",
        }
    }
}

/// Escapa texto para insertarlo en HTML (contenido o valor de atributo).
pub fn escapar(texto: &str) -> String {
    let mut salida = String::with_capacity(texto.len());
    for c in texto.chars() {
        match c {
            '&' => salida.push_str("&amp;"),
            '<' => salida.push_str("&lt;"),
            '>' => salida.push_str("&gt;"),
            '"' => salida.push_str("&quot;"),
            '\'' => salida.push_str("&#39;"),
            _ => salida.push(c),
        }
    }
    salida
}

/// Serializa un árbol a HTML.
///
/// Todo texto pasa por [`escapar`]; no hay forma de inyectar marcado desde los
/// datos. Las acciones de los botones viajan como `data-fg-accion`, que es lo
/// que el runtime engancha a un listener real.
pub fn render(nodo: &Nodo) -> String {
    let mut salida = String::new();
    escribir(nodo, &mut salida, None);
    salida
}

/// Render con los valores del estado (SSR o re-render tras un evento).
pub fn render_con_estado(nodo: &Nodo, estado: &Estado) -> String {
    let mut salida = String::new();
    escribir(nodo, &mut salida, Some(estado));
    salida
}

/// Documento completo, útil para SSR.
pub fn documento(titulo: &str, raiz: &Nodo, hoja_estilos: Option<&str>) -> String {
    let estilos = hoja_estilos
        .map(|href| format!("<link rel=\"stylesheet\" href=\"{}\">", escapar(href)))
        .unwrap_or_default();
    format!(
        "<!doctype html><html lang=\"es\"><head><meta charset=\"utf-8\">\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\
         <title>{}</title>{}</head><body>{}</body></html>",
        escapar(titulo),
        estilos,
        render(raiz)
    )
}

fn escribir(nodo: &Nodo, salida: &mut String, estado: Option<&Estado>) {
    match nodo {
        Nodo::Contenedor { hijos, clase } => {
            if clase.is_empty() {
                salida.push_str("<div>");
            } else {
                let _ = write!(salida, "<div class=\"{}\">", escapar(clase));
            }
            for hijo in hijos {
                escribir(hijo, salida, estado);
            }
            salida.push_str("</div>");
        }
        Nodo::Texto(texto) => salida.push_str(&escapar(texto)),
        Nodo::Variable(nombre) => {
            let valor = estado
                .and_then(|e| e.obtener(nombre))
                .map(escapar)
                .unwrap_or_default();
            let _ = write!(
                salida,
                "<span data-fg-variable=\"{}\">{}</span>",
                escapar(nombre),
                valor
            );
        }
        Nodo::Encabezado { nivel, texto } => {
            let n = (*nivel).clamp(1, 6);
            let _ = write!(salida, "<h{n}>{}</h{n}>", escapar(texto));
        }
        Nodo::Parrafo(texto) => {
            let _ = write!(salida, "<p>{}</p>", escapar(texto));
        }
        Nodo::Enlace { href, texto } => {
            let _ = write!(
                salida,
                "<a href=\"{}\">{}</a>",
                escapar(href),
                escapar(texto)
            );
        }
        Nodo::Boton {
            texto,
            accion,
            variante,
        } => {
            let clase = match variante {
                VarianteBoton::Primario => "fg-boton",
                VarianteBoton::Secundario => "fg-boton fg-boton-secundario",
            };
            let _ = write!(
                salida,
                "<button type=\"button\" class=\"{}\" data-fg-accion=\"{}\">{}</button>",
                clase,
                escapar(accion),
                escapar(texto)
            );
        }
        Nodo::Entrada {
            nombre,
            tipo,
            valor,
            marcador,
            requerido,
        } => {
            let id = format!("fg-{nombre}");
            let requerido = if *requerido { " required" } else { "" };
            let valor = estado
                .and_then(|e| e.obtener(nombre))
                .unwrap_or(valor);
            let _ = write!(
                salida,
                "<label for=\"{id}\">{}</label>\
                 <input id=\"{id}\" name=\"{}\" data-fg-variable=\"{}\" type=\"{}\" value=\"{}\" placeholder=\"{}\"{requerido}>",
                escapar(nombre),
                escapar(nombre),
                escapar(nombre),
                tipo.html(),
                escapar(valor),
                escapar(marcador)
            );
        }
        Nodo::Casilla {
            nombre,
            etiqueta,
            marcado,
        } => {
            let id = format!("fg-{nombre}");
            let marcado = if *marcado { " checked" } else { "" };
            let _ = write!(
                salida,
                "<input id=\"{id}\" name=\"{}\" data-fg-variable=\"{}\" type=\"checkbox\"{marcado}>\
                 <label for=\"{id}\">{}</label>",
                escapar(nombre),
                escapar(nombre),
                escapar(etiqueta)
            );
        }
        Nodo::Lista { items, clase } => {
            if clase.is_empty() {
                salida.push_str("<ul>");
            } else {
                let _ = write!(salida, "<ul class=\"{}\">", escapar(clase));
            }
            for item in items {
                salida.push_str("<li>");
                escribir(item, salida, estado);
                salida.push_str("</li>");
            }
            salida.push_str("</ul>");
        }
        Nodo::Tabla { cabeceras, filas } => {
            salida.push_str("<table>");
            if !cabeceras.is_empty() {
                salida.push_str("<thead><tr>");
                for cabecera in cabeceras {
                    let _ = write!(salida, "<th>{}</th>", escapar(cabecera));
                }
                salida.push_str("</tr></thead>");
            }
            salida.push_str("<tbody>");
            for fila in filas {
                salida.push_str("<tr>");
                for celda in fila {
                    salida.push_str("<td>");
                    escribir(celda, salida, estado);
                    salida.push_str("</td>");
                }
                salida.push_str("</tr>");
            }
            salida.push_str("</tbody></table>");
        }
        Nodo::Imagen { src, alt } => {
            let _ = write!(
                salida,
                "<img src=\"{}\" alt=\"{}\">",
                escapar(src),
                escapar(alt)
            );
        }
        Nodo::Separador => salida.push_str("<hr>"),
    }
}

/// Nombres de los atributos con los que el runtime del navegador engancha
/// eventos. Se exponen para que el JS/wasm no los escriba a mano.
pub mod eventos {
    /// Acción declarativa de un botón (`data-fg-accion="guardar"`).
    pub const ACCION: &str = "data-fg-accion";
    /// Prefijo de los ids generados para etiquetas y campos.
    pub const PREFIJO_ID: &str = "fg-";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapa_los_cinco_caracteres() {
        assert_eq!(escapar("<&>\"'"), "&lt;&amp;&gt;&quot;&#39;");
        assert_eq!(escapar("normal"), "normal");
    }

    #[test]
    fn texto_se_escapa_siempre() {
        let html = render(&Nodo::Texto("<script>alert(1)</script>".into()));
        assert_eq!(html, "&lt;script&gt;alert(1)&lt;/script&gt;");
    }

    #[test]
    fn contenedor_con_y_sin_clase() {
        let con = Nodo::Contenedor {
            hijos: vec![Nodo::Texto("a".into())],
            clase: "caja".into(),
        };
        assert_eq!(render(&con), "<div class=\"caja\">a</div>");
        let sin = Nodo::Contenedor {
            hijos: vec![],
            clase: String::new(),
        };
        assert_eq!(render(&sin), "<div></div>");
    }

    #[test]
    fn encabezado_limita_el_nivel() {
        let h = |n| {
            render(&Nodo::Encabezado {
                nivel: n,
                texto: "t".into(),
            })
        };
        assert_eq!(h(0), "<h1>t</h1>");
        assert_eq!(h(3), "<h3>t</h3>");
        assert_eq!(h(9), "<h6>t</h6>");
    }

    #[test]
    fn boton_lleva_accion_declarativa() {
        let html = render(&Nodo::Boton {
            texto: "Guardar".into(),
            accion: "guardar".into(),
            variante: VarianteBoton::Primario,
        });
        assert_eq!(
            html,
            "<button type=\"button\" class=\"fg-boton\" data-fg-accion=\"guardar\">Guardar</button>"
        );
    }

    #[test]
    fn entrada_tiene_etiqueta_e_id() {
        let html = render(&Nodo::Entrada {
            nombre: "email".into(),
            tipo: TipoEntrada::Email,
            valor: "a@b".into(),
            marcador: "tu@correo".into(),
            requerido: true,
        });
        assert!(html.contains("<label for=\"fg-email\">email</label>"));
        assert!(html.contains("id=\"fg-email\" name=\"email\" data-fg-variable=\"email\" type=\"email\""));
        assert!(html.contains("value=\"a@b\""));
        assert!(html.contains(" required"));
    }

    #[test]
    fn entrada_escapa_valor_y_marcador() {
        let html = render(&Nodo::Entrada {
            nombre: "x".into(),
            tipo: TipoEntrada::Texto,
            valor: "\"<a>\"".into(),
            marcador: "\"b\"".into(),
            requerido: false,
        });
        assert!(html.contains("&quot;&lt;a&gt;&quot;"));
        assert!(html.contains("&quot;b&quot;"));
        assert!(!html.contains(" required"));
    }

    #[test]
    fn casilla_con_etiqueta() {
        let html = render(&Nodo::Casilla {
            nombre: "hecha".into(),
            etiqueta: "Hecha".into(),
            marcado: true,
        });
        assert!(html.contains("type=\"checkbox\" checked"));
        assert!(html.contains("<label for=\"fg-hecha\">Hecha</label>"));
    }

    #[test]
    fn lista_y_tabla() {
        let lista = render(&Nodo::Lista {
            items: vec![Nodo::Texto("a".into()), Nodo::Texto("b".into())],
            clase: "tareas".into(),
        });
        assert_eq!(lista, "<ul class=\"tareas\"><li>a</li><li>b</li></ul>");

        let tabla = render(&Nodo::Tabla {
            cabeceras: vec!["a".into()],
            filas: vec![vec![Nodo::Texto("1".into())]],
        });
        assert_eq!(
            tabla,
            "<table><thead><tr><th>a</th></tr></thead><tbody><tr><td>1</td></tr></tbody></table>"
        );
    }

    #[test]
    fn imagen_exige_alt() {
        let html = render(&Nodo::Imagen {
            src: "/a.png".into(),
            alt: "un gato".into(),
        });
        assert_eq!(html, "<img src=\"/a.png\" alt=\"un gato\">");
    }

    #[test]
    fn documento_completo() {
        let raiz = Nodo::Contenedor {
            hijos: vec![Nodo::Encabezado {
                nivel: 1,
                texto: "T & T".into(),
            }],
            clase: String::new(),
        };
        let html = documento("Tareas", &raiz, Some("/publico/estilo.css"));
        assert!(html.starts_with("<!doctype html><html lang=\"es\">"));
        assert!(html.contains("<title>Tareas</title>"));
        assert!(html.contains("<link rel=\"stylesheet\" href=\"/publico/estilo.css\">"));
        assert!(html.contains("<h1>T &amp; T</h1>"));
        assert!(!documento("t", &raiz, None).contains("stylesheet"));
    }


    #[test]
    fn estado_y_eventos() {
        let mut estado = Estado::nuevo();
        estado.fijar("nombre", "ana");
        assert_eq!(estado.obtener("nombre"), Some("ana"));
        assert_eq!(estado.obtener("otra"), None);

        // Cambio de un campo: actualiza y pide re-render.
        assert!(aplicar(
            &Evento::Cambio {
                variable: "nombre".into(),
                valor: "carla".into()
            },
            &mut estado
        ));
        assert_eq!(estado.obtener("nombre"), Some("carla"));

        // Casilla: booleano.
        assert!(aplicar(
            &Evento::Marcar {
                variable: "hecha".into(),
                marcado: true
            },
            &mut estado
        ));
        assert!(estado.booleano("hecha"));
        estado.fijar("hecha", "falso");
        assert!(!estado.booleano("hecha"));

        // Un clic no toca el estado (la lógica la resuelve el programa).
        assert!(!aplicar(&Evento::Click("guardar".into()), &mut estado));
    }

    #[test]
    fn render_con_estado_prellena_valores() {
        let arbol = Nodo::Contenedor {
            hijos: vec![
                Nodo::Variable("saludo".into()),
                Nodo::Entrada {
                    nombre: "nombre".into(),
                    tipo: TipoEntrada::Texto,
                    valor: "por defecto".into(),
                    marcador: String::new(),
                    requerido: false,
                },
            ],
            clase: String::new(),
        };
        let mut estado = Estado::nuevo();
        estado.fijar("saludo", "hola <b>");
        estado.fijar("nombre", "ana");

        let html = render_con_estado(&arbol, &estado);
        assert!(html.contains("<span data-fg-variable=\"saludo\">hola &lt;b&gt;</span>"));
        assert!(html.contains("value=\"ana\""));

        // Sin estado, el input conserva su valor por defecto y la variable va vacía.
        let plano = render(&arbol);
        assert!(plano.contains("<span data-fg-variable=\"saludo\"></span>"));
        assert!(plano.contains("value=\"por defecto\""));
    }

    #[test]
    fn acciones_y_variables_del_arbol() {
        let arbol = Nodo::Contenedor {
            hijos: vec![
                Nodo::Boton {
                    texto: "Guardar".into(),
                    accion: "guardar".into(),
                    variante: VarianteBoton::Primario,
                },
                Nodo::Lista {
                    items: vec![Nodo::Boton {
                        texto: "Borrar".into(),
                        accion: "borrar:7".into(),
                        variante: VarianteBoton::Secundario,
                    }],
                    clase: String::new(),
                },
                Nodo::Tabla {
                    cabeceras: vec![],
                    filas: vec![vec![Nodo::Entrada {
                        nombre: "filtro".into(),
                        tipo: TipoEntrada::Busqueda,
                        valor: String::new(),
                        marcador: String::new(),
                        requerido: false,
                    }]],
                },
                Nodo::Entrada {
                    nombre: "filtro".into(),
                    tipo: TipoEntrada::Busqueda,
                    valor: String::new(),
                    marcador: String::new(),
                    requerido: false,
                },
            ],
            clase: String::new(),
        };
        assert_eq!(acciones(&arbol), vec!["guardar", "borrar:7"]);
        // Sin repetir aunque aparezca dos veces.
        assert_eq!(variables(&arbol), vec!["filtro"]);
    }

    #[test]
    fn arbol_anidado() {
        let arbol = Nodo::Lista {
            items: vec![Nodo::Contenedor {
                hijos: vec![
                    Nodo::Texto("comprar".into()),
                    Nodo::Boton {
                        texto: "hecha".into(),
                        accion: "hecha:1".into(),
                        variante: VarianteBoton::Secundario,
                    },
                ],
                clase: String::new(),
            }],
            clase: String::new(),
        };
        assert_eq!(
            render(&arbol),
            "<ul><li><div>comprar<button type=\"button\" class=\"fg-boton fg-boton-secundario\" data-fg-accion=\"hecha:1\">hecha</button></div></li></ul>"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════
// ESTADO Y EVENTOS (lo que el runtime del navegador mueve)
// ═══════════════════════════════════════════════════════════════════

use std::collections::BTreeMap;

/// Estado de la interfaz: nombre de variable → valor.
///
/// Es lo que el runtime actualiza cuando el usuario escribe o hace clic, y de
/// donde sale el siguiente render. Tipos: todo texto (un input siempre lo es);
/// los booleanos se guardan como `"verdadero"`/`"falso"`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Estado {
    valores: BTreeMap<String, String>,
}

impl Estado {
    pub fn nuevo() -> Self {
        Estado::default()
    }

    pub fn fijar(&mut self, nombre: &str, valor: impl Into<String>) {
        self.valores.insert(nombre.to_string(), valor.into());
    }

    /// `true` para `"verdadero"`, `"true"`, `"1"` o `"si"`.
    pub fn fijar_booleano(&mut self, nombre: &str, valor: bool) {
        self.fijar(nombre, if valor { "verdadero" } else { "falso" });
    }

    pub fn obtener(&self, nombre: &str) -> Option<&str> {
        self.valores.get(nombre).map(String::as_str)
    }

    pub fn booleano(&self, nombre: &str) -> bool {
        matches!(
            self.obtener(nombre),
            Some("verdadero") | Some("true") | Some("1") | Some("si")
        )
    }

    pub fn valores(&self) -> &BTreeMap<String, String> {
        &self.valores
    }
}

/// Evento que produce el navegador (o un test).
#[derive(Debug, Clone, PartialEq)]
pub enum Evento {
    /// Clic en un botón con acción declarativa (`data-fg-accion`).
    Click(String),
    /// Cambio de valor en un campo ligado a una variable.
    Cambio { variable: String, valor: String },
    /// Casilla marcada/desmarcada.
    Marcar { variable: String, marcado: bool },
}

/// Aplica un evento al estado. Devuelve `true` si hay que re-renderizar.
///
/// Las acciones de clic no tocan el estado: las resuelve el programa Forja
/// (son su lógica); acá sólo se informa que hubo un clic.
pub fn aplicar(evento: &Evento, estado: &mut Estado) -> bool {
    match evento {
        Evento::Click(_) => false,
        Evento::Cambio { variable, valor } => {
            estado.fijar(variable, valor.clone());
            true
        }
        Evento::Marcar { variable, marcado } => {
            estado.fijar_booleano(variable, *marcado);
            true
        }
    }
}

/// Acciones declarativas del árbol, en orden (para enganchar delegación de clics).
pub fn acciones(nodo: &Nodo) -> Vec<String> {
    let mut salida = Vec::new();
    recolectar_acciones(nodo, &mut salida);
    salida
}

fn recolectar_acciones(nodo: &Nodo, salida: &mut Vec<String>) {
    match nodo {
        Nodo::Boton { accion, .. } => salida.push(accion.clone()),
        Nodo::Contenedor { hijos, .. } | Nodo::Lista { items: hijos, .. } => {
            for hijo in hijos {
                recolectar_acciones(hijo, salida);
            }
        }
        Nodo::Tabla { filas, .. } => {
            for fila in filas {
                for celda in fila {
                    recolectar_acciones(celda, salida);
                }
            }
        }
        _ => {}
    }
}

/// Variables ligadas a campos (inputs y casillas), en orden y sin repetir.
pub fn variables(nodo: &Nodo) -> Vec<String> {
    let mut salida = Vec::new();
    recolectar_variables(nodo, &mut salida);
    salida.dedup();
    salida
}

fn recolectar_variables(nodo: &Nodo, salida: &mut Vec<String>) {
    match nodo {
        Nodo::Entrada { nombre, .. } | Nodo::Casilla { nombre, .. } => {
            if !salida.contains(nombre) {
                salida.push(nombre.clone());
            }
        }
        Nodo::Contenedor { hijos, .. } | Nodo::Lista { items: hijos, .. } => {
            for hijo in hijos {
                recolectar_variables(hijo, salida);
            }
        }
        Nodo::Tabla { filas, .. } => {
            for fila in filas {
                for celda in fila {
                    recolectar_variables(celda, salida);
                }
            }
        }
        _ => {}
    }
}
