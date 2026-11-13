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
        deshabilitado: bool,
    },
    /// Botón sólo icono: el texto accesible va en `aria-label`.
    BotonIcono {
        icono: String,
        etiqueta: String,
        accion: String,
        deshabilitado: bool,
    },
    /// Formulario real (`<form>`): Enter envía y, sin JS, hace un POST normal.
    /// `accion` vacía deja el formulario en manos del runtime.
    Formulario {
        accion: String,
        metodo: String,
        hijos: Vec<Nodo>,
    },
    /// Campo de texto con etiqueta accesible.
    Entrada {
        nombre: String,
        tipo: TipoEntrada,
        valor: String,
        marcador: String,
        requerido: bool,
        /// Mensaje de validación: marca el campo inválido y lo anuncia.
        error: Option<String>,
    },
    /// Casilla con etiqueta.
    Casilla {
        nombre: String,
        etiqueta: String,
        marcado: bool,
    },
    /// Lista.
    Lista { items: Vec<Nodo>, clase: String },
    /// Tabla con cabeceras y filas. Con `orden`, las cabeceras son botones que
    /// emiten `orden:<columna>` y llevan `aria-sort`.
    Tabla {
        cabeceras: Vec<String>,
        filas: Vec<Vec<Nodo>>,
        orden: Option<(String, Orden)>,
    },
    /// Imagen con texto alternativo obligatorio (accesibilidad).
    Imagen { src: String, alt: String },
    /// Separador.
    Separador,
    /// Desplegable con etiqueta accesible. `opciones` es (valor, etiqueta).
    Seleccion {
        nombre: String,
        etiqueta: String,
        opciones: Vec<(String, String)>,
        seleccionada: String,
    },
    /// Área de texto multilínea.
    AreaTexto {
        nombre: String,
        valor: String,
        marcador: String,
        filas: u32,
    },
    /// Grupo de campos con leyenda (`fieldset`/`legend`).
    GrupoCampos { leyenda: String, hijos: Vec<Nodo> },
    /// Barra de progreso con etiqueta accesible.
    Progreso {
        valor: f64,
        maximo: f64,
        etiqueta: String,
    },
    /// Insignia/chip de estado.
    Insignia { texto: String, tono: Tono },
    /// Tarjeta: sección con título.
    Tarjeta { titulo: String, hijos: Vec<Nodo> },
    /// Mensaje de aviso, con `role` según la gravedad.
    Aviso { texto: String, nivel: NivelAviso },
    /// Navegación con enlaces.
    Navegacion { enlaces: Vec<(String, String)> },
    /// Diálogo nativo (`<dialog>`), accesible por el navegador.
    Dialogo {
        id: String,
        titulo: String,
        hijos: Vec<Nodo>,
        abierto: bool,
    },
    /// Indicador de carga (`aria-busy`).
    Cargando { texto: String },
    /// Pestañas (patrón ARIA `tablist`/`tab`/`tabpanel`). La activa sale de la
    /// variable ligada; cada pestaña dispara `tab:<variable>:<valor>`.
    Pestanas {
        variable: String,
        items: Vec<(String, String, Vec<Nodo>)>,
    },
    /// Lista de opciones con selección única (ARIA `listbox`). La elegida sale
    /// de la variable ligada; cada opción dispara `sel:<variable>:<valor>`.
    ListaSeleccion {
        variable: String,
        etiqueta: String,
        /// Valor marcado si el programa todavía no tiene la variable.
        por_defecto: String,
        opciones: Vec<(String, String)>,
    },
    /// Bloque plegable nativo (`<details>`/`<summary>`): foco y teclado gratis.
    Plegable {
        titulo: String,
        hijos: Vec<Nodo>,
        abierto: bool,
    },
}

/// Sentido del orden de una tabla.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orden {
    Ascendente,
    Descendente,
}

/// Tono visual de una insignia (el color lo pone el CSS).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tono {
    Neutro,
    Exito,
    Aviso,
    Error,
}

/// Gravedad de un aviso: define el `role` para lectores de pantalla.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NivelAviso {
    Informacion,
    Advertencia,
    Error,
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
            deshabilitado,
        } => {
            let clase = match variante {
                VarianteBoton::Primario => "fg-boton",
                VarianteBoton::Secundario => "fg-boton fg-boton-secundario",
            };
            let _ = write!(
                salida,
                "<button type=\"button\" class=\"{}\" data-fg-accion=\"{}\"{}>{}</button>",
                clase,
                escapar(accion),
                if *deshabilitado { " disabled" } else { "" },
                escapar(texto)
            );
        }
        Nodo::BotonIcono {
            icono,
            etiqueta,
            accion,
            deshabilitado,
        } => {
            let _ = write!(
                salida,
                "<button type=\"button\" class=\"fg-boton fg-boton-icono\" aria-label=\"{}\" data-fg-accion=\"{}\"{}>{}</button>",
                escapar(etiqueta),
                escapar(accion),
                if *deshabilitado { " disabled" } else { "" },
                escapar(icono)
            );
        }
        Nodo::Formulario {
            accion,
            metodo,
            hijos,
        } => {
            let metodo_form = if metodo.is_empty() { "post" } else { metodo };
            let atributo_accion = if accion.is_empty() {
                String::new()
            } else {
                format!(" action=\"{}\"", escapar(accion))
            };
            let _ = write!(
                salida,
                "<form class=\"fg-formulario\" method=\"{}\"{atributo_accion} data-fg-formulario=\"1\">",
                escapar(metodo_form)
            );
            for hijo in hijos {
                escribir(hijo, salida, estado);
            }
            salida.push_str("</form>");
        }
        Nodo::Entrada {
            nombre,
            tipo,
            valor,
            marcador,
            requerido,
            error,
        } => {
            let id = format!("fg-{nombre}");
            let requerido = if *requerido { " required" } else { "" };
            let valor = estado.and_then(|e| e.obtener(nombre)).unwrap_or(valor);
            let (invalidos, descrito, mensaje) = match error {
                Some(texto) => (
                    " aria-invalid=\"true\"",
                    format!(" aria-describedby=\"fg-{nombre}-error\""),
                    format!(
                        "<p class=\"fg-error\" id=\"fg-{nombre}-error\" role=\"alert\">{}</p>",
                        escapar(texto)
                    ),
                ),
                None => ("", String::new(), String::new()),
            };
            let _ = write!(
                salida,
                "<label for=\"{id}\">{}</label><input id=\"{id}\" name=\"{}\" data-fg-variable=\"{}\" type=\"{}\" value=\"{}\" placeholder=\"{}\"{requerido}{invalidos}{descrito}>{}",
                escapar(nombre),
                escapar(nombre),
                escapar(nombre),
                tipo.html(),
                escapar(valor),
                escapar(marcador),
                mensaje
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
        Nodo::Tabla {
            cabeceras,
            filas,
            orden,
        } => {
            salida.push_str("<table>");
            if !cabeceras.is_empty() {
                salida.push_str("<thead><tr>");
                for cabecera in cabeceras {
                    if orden.is_some() {
                        // Cabecera ordenable: `aria-sort` + acción `orden:<columna>`.
                        let sentido = match orden {
                            Some((columna, sentido)) if columna == cabecera => match sentido {
                                Orden::Ascendente => "ascending",
                                Orden::Descendente => "descending",
                            },
                            _ => "none",
                        };
                        let _ = write!(
                            salida,
                            "<th aria-sort=\"{sentido}\"><button type=\"button\" class=\"fg-ordenar\" data-fg-accion=\"orden:{}\">{}</button></th>",
                            escapar(cabecera),
                            escapar(cabecera)
                        );
                    } else {
                        let _ = write!(salida, "<th>{}</th>", escapar(cabecera));
                    }
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
        Nodo::Seleccion {
            nombre,
            etiqueta,
            opciones,
            seleccionada,
        } => {
            let id = format!("fg-{nombre}");
            let elegida = estado
                .and_then(|e| e.obtener(nombre))
                .unwrap_or(seleccionada);
            let _ = write!(
                salida,
                "<label for=\"{id}\">{}</label>                 <select id=\"{id}\" name=\"{}\" data-fg-variable=\"{}\">",
                escapar(etiqueta),
                escapar(nombre),
                escapar(nombre)
            );
            for (valor, texto) in opciones {
                let marcada = if valor == elegida { " selected" } else { "" };
                let _ = write!(
                    salida,
                    "<option value=\"{}\"{marcada}>{}</option>",
                    escapar(valor),
                    escapar(texto)
                );
            }
            salida.push_str("</select>");
        }
        Nodo::AreaTexto {
            nombre,
            valor,
            marcador,
            filas,
        } => {
            let id = format!("fg-{nombre}");
            let contenido = estado.and_then(|e| e.obtener(nombre)).unwrap_or(valor);
            let _ = write!(
                salida,
                "<label for=\"{id}\">{}</label>                 <textarea id=\"{id}\" name=\"{}\" data-fg-variable=\"{}\" rows=\"{}\" placeholder=\"{}\">{}</textarea>",
                escapar(nombre),
                escapar(nombre),
                escapar(nombre),
                (*filas).clamp(1, 40),
                escapar(marcador),
                escapar(contenido)
            );
        }
        Nodo::GrupoCampos { leyenda, hijos } => {
            let _ = write!(salida, "<fieldset><legend>{}</legend>", escapar(leyenda));
            for hijo in hijos {
                escribir(hijo, salida, estado);
            }
            salida.push_str("</fieldset>");
        }
        Nodo::Progreso {
            valor,
            maximo,
            etiqueta,
        } => {
            let maximo = if *maximo <= 0.0 { 1.0 } else { *maximo };
            let valor = valor.clamp(0.0, maximo);
            let porcentaje = ((valor / maximo) * 100.0).round() as i64;
            let _ = write!(
                salida,
                "<progress value=\"{}\" max=\"{}\" aria-label=\"{}\" aria-valuenow=\"{}\" aria-valuemin=\"0\" aria-valuemax=\"100\"></progress>",
                valor, maximo, escapar(etiqueta), porcentaje
            );
        }
        Nodo::Insignia { texto, tono } => {
            let clase = match tono {
                Tono::Neutro => "fg-insignia",
                Tono::Exito => "fg-insignia fg-insignia-exito",
                Tono::Aviso => "fg-insignia fg-insignia-aviso",
                Tono::Error => "fg-insignia fg-insignia-error",
            };
            let _ = write!(salida, "<span class=\"{clase}\">{}</span>", escapar(texto));
        }
        Nodo::Tarjeta { titulo, hijos } => {
            let _ = write!(
                salida,
                "<section class=\"fg-tarjeta\"><h2>{}</h2>",
                escapar(titulo)
            );
            for hijo in hijos {
                escribir(hijo, salida, estado);
            }
            salida.push_str("</section>");
        }
        Nodo::Aviso { texto, nivel } => {
            let (clase, role) = match nivel {
                NivelAviso::Informacion => ("fg-aviso fg-aviso-info", "note"),
                NivelAviso::Advertencia => ("fg-aviso fg-aviso-aviso", "status"),
                NivelAviso::Error => ("fg-aviso fg-aviso-error", "alert"),
            };
            let _ = write!(
                salida,
                "<p class=\"{clase}\" role=\"{role}\">{}</p>",
                escapar(texto)
            );
        }
        Nodo::Navegacion { enlaces } => {
            salida.push_str("<nav><ul>");
            for (href, texto) in enlaces {
                let _ = write!(
                    salida,
                    "<li><a href=\"{}\">{}</a></li>",
                    escapar(href),
                    escapar(texto)
                );
            }
            salida.push_str("</ul></nav>");
        }
        Nodo::Dialogo {
            id,
            titulo,
            hijos,
            abierto,
        } => {
            let abierto = if *abierto { " open" } else { "" };
            let _ = write!(
                salida,
                "<dialog id=\"fg-dialogo-{}\" aria-label=\"{}\"{abierto}><h2>{}</h2>",
                escapar(id),
                escapar(titulo),
                escapar(titulo)
            );
            for hijo in hijos {
                escribir(hijo, salida, estado);
            }
            salida.push_str("</dialog>");
        }
        Nodo::Pestanas { variable, items } => {
            let activa = estado
                .and_then(|e| e.obtener(variable))
                .map(|v| v.to_string())
                .unwrap_or_else(|| {
                    items
                        .first()
                        .map(|(valor, _, _)| valor.clone())
                        .unwrap_or_default()
                });
            let _ = write!(
                salida,
                "<div class=\"fg-pestanas\" role=\"tablist\" data-fg-tablist=\"{}\">",
                escapar(variable)
            );
            for (valor, etiqueta, _) in items {
                let seleccionada = valor == &activa;
                let _ = write!(
                    salida,
                    "<button type=\"button\" role=\"tab\" class=\"fg-pestana\" aria-selected=\"{}\" data-fg-accion=\"tab:{}:{}\">{}</button>",
                    if seleccionada { "true" } else { "false" },
                    escapar(variable),
                    escapar(valor),
                    escapar(etiqueta)
                );
            }
            salida.push_str("</div>");
            for (valor, etiqueta, hijos) in items {
                let visible = valor == &activa;
                let _ = write!(
                    salida,
                    "<div class=\"fg-panel\" role=\"tabpanel\" data-fg-panel=\"{}\" data-fg-panel-var=\"{}\" aria-label=\"{}\"{}>",
                    escapar(valor),
                    escapar(variable),
                    escapar(etiqueta),
                    if visible { "" } else { " hidden" }
                );
                for hijo in hijos {
                    escribir(hijo, salida, estado);
                }
                salida.push_str("</div>");
            }
        }
        Nodo::ListaSeleccion {
            variable,
            etiqueta,
            por_defecto,
            opciones,
        } => {
            let elegida = estado
                .and_then(|e| e.obtener(variable))
                .map(|v| v.to_string())
                .unwrap_or_else(|| por_defecto.clone());
            let _ = write!(
                salida,
                "<span id=\"fg-{}-etiqueta\" class=\"fg-etiqueta-grupo\">{}</span>                 <ul class=\"fg-opciones\" role=\"listbox\" aria-labelledby=\"fg-{}-etiqueta\" data-fg-listbox=\"{}\">",
                escapar(variable),
                escapar(etiqueta),
                escapar(variable),
                escapar(variable)
            );
            for (valor, texto) in opciones {
                let seleccionada = valor == &elegida;
                let _ = write!(
                    salida,
                    "<li><button type=\"button\" role=\"option\" class=\"fg-opcion\" aria-selected=\"{}\" data-fg-accion=\"sel:{}:{}\">{}</button></li>",
                    if seleccionada { "true" } else { "false" },
                    escapar(variable),
                    escapar(valor),
                    escapar(texto)
                );
            }
            salida.push_str("</ul>");
        }
        Nodo::Plegable {
            titulo,
            hijos,
            abierto,
        } => {
            let _ = write!(
                salida,
                "<details class=\"fg-plegable\"{}><summary>{}</summary>",
                if *abierto { " open" } else { "" },
                escapar(titulo)
            );
            for hijo in hijos {
                escribir(hijo, salida, estado);
            }
            salida.push_str("</details>");
        }
        Nodo::Cargando { texto } => {
            let _ = write!(
                salida,
                "<p class=\"fg-cargando\" aria-busy=\"true\" role=\"status\">{}</p>",
                escapar(texto)
            );
        }
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
            deshabilitado: false,
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
            error: None,
        });
        assert!(html.contains("<label for=\"fg-email\">email</label>"));
        assert!(html
            .contains("id=\"fg-email\" name=\"email\" data-fg-variable=\"email\" type=\"email\""));
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
            error: None,
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
            orden: None,
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
                    error: None,
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
                    deshabilitado: false,
                },
                Nodo::Lista {
                    items: vec![Nodo::Boton {
                        texto: "Borrar".into(),
                        accion: "borrar:7".into(),
                        variante: VarianteBoton::Secundario,
                        deshabilitado: false,
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
                        error: None,
                    }]],
                    orden: None,
                },
                Nodo::Entrada {
                    nombre: "filtro".into(),
                    tipo: TipoEntrada::Busqueda,
                    valor: String::new(),
                    marcador: String::new(),
                    requerido: false,
                    error: None,
                },
            ],
            clase: String::new(),
        };
        assert_eq!(acciones(&arbol), vec!["guardar", "borrar:7"]);
        // Sin repetir aunque aparezca dos veces.
        assert_eq!(variables(&arbol), vec!["filtro"]);
    }

    #[test]
    fn seleccion_con_opciones_y_valor_del_estado() {
        let nodo = Nodo::Seleccion {
            nombre: "pais".into(),
            etiqueta: "País".into(),
            opciones: vec![
                ("ar".into(), "Argentina".into()),
                ("uy".into(), "Uruguay".into()),
            ],
            seleccionada: "ar".into(),
        };
        let html = render(&nodo);
        assert!(html.contains("<label for=\"fg-pais\">País</label>"));
        assert!(html.contains("data-fg-variable=\"pais\""));
        assert!(html.contains("<option value=\"ar\" selected>Argentina</option>"));
        assert!(html.contains("<option value=\"uy\">Uruguay</option>"));

        let mut estado = Estado::nuevo();
        estado.fijar("pais", "uy");
        let con_estado = render_con_estado(&nodo, &estado);
        assert!(con_estado.contains("<option value=\"uy\" selected>"));
        assert!(!con_estado.contains("value=\"ar\" selected"));
    }

    #[test]
    fn area_texto_con_filas_y_valor() {
        let nodo = Nodo::AreaTexto {
            nombre: "notas".into(),
            valor: "hola".into(),
            marcador: "escribí acá".into(),
            filas: 0,
        };
        let html = render(&nodo);
        assert!(html.contains("<textarea id=\"fg-notas\" name=\"notas\""));
        assert!(html.contains("rows=\"1\"")); // se limita a 1 como mínimo
        assert!(html.contains(">hola</textarea>"));

        let mut estado = Estado::nuevo();
        estado.fijar("notas", "<b>x</b>");
        assert!(render_con_estado(&nodo, &estado).contains("&lt;b&gt;x&lt;/b&gt;</textarea>"));
    }

    #[test]
    fn grupo_campos_progreso_e_insignias() {
        let grupo = render(&Nodo::GrupoCampos {
            leyenda: "Datos".into(),
            hijos: vec![Nodo::Parrafo("x".into())],
        });
        assert_eq!(grupo, "<fieldset><legend>Datos</legend><p>x</p></fieldset>");

        let progreso = render(&Nodo::Progreso {
            valor: 3.0,
            maximo: 4.0,
            etiqueta: "avance".into(),
        });
        assert!(progreso.contains("value=\"3\" max=\"4\""));
        assert!(progreso.contains("aria-label=\"avance\""));
        assert!(progreso.contains("aria-valuenow=\"75\""));

        // maximo 0 no divide por cero.
        let cero = render(&Nodo::Progreso {
            valor: 1.0,
            maximo: 0.0,
            etiqueta: "x".into(),
        });
        assert!(cero.contains("max=\"1\""));

        assert!(render(&Nodo::Insignia {
            texto: "listo".into(),
            tono: Tono::Exito
        })
        .contains("class=\"fg-insignia fg-insignia-exito\""));
    }

    #[test]
    fn tarjeta_aviso_navegacion_dialogo_y_carga() {
        let tarjeta = render(&Nodo::Tarjeta {
            titulo: "Hoy".into(),
            hijos: vec![Nodo::Parrafo("2 tareas".into())],
        });
        assert!(tarjeta.starts_with("<section class=\"fg-tarjeta\"><h2>Hoy</h2>"));
        assert!(tarjeta.ends_with("</section>"));

        // El rol cambia según la gravedad: un lector de pantalla lo anuncia.
        assert!(render(&Nodo::Aviso {
            texto: "falló".into(),
            nivel: NivelAviso::Error
        })
        .contains("role=\"alert\""));
        assert!(render(&Nodo::Aviso {
            texto: "ojo".into(),
            nivel: NivelAviso::Advertencia
        })
        .contains("role=\"status\""));
        assert!(render(&Nodo::Aviso {
            texto: "nota".into(),
            nivel: NivelAviso::Informacion
        })
        .contains("role=\"note\""));

        let nav = render(&Nodo::Navegacion {
            enlaces: vec![("/a".into(), "A".into()), ("/b".into(), "B".into())],
        });
        assert_eq!(
            nav,
            "<nav><ul><li><a href=\"/a\">A</a></li><li><a href=\"/b\">B</a></li></ul></nav>"
        );

        let dialogo = render(&Nodo::Dialogo {
            id: "confirmar".into(),
            titulo: "¿Seguro?".into(),
            hijos: vec![Nodo::Boton {
                texto: "Sí".into(),
                accion: "confirmar".into(),
                variante: VarianteBoton::Primario,
                deshabilitado: false,
            }],
            abierto: true,
        });
        assert!(dialogo.contains("id=\"fg-dialogo-confirmar\""));
        assert!(dialogo.contains(" aria-label=\"¿Seguro?\" open>"));
        assert!(dialogo.contains("data-fg-accion=\"confirmar\""));

        assert!(render(&Nodo::Cargando {
            texto: "cargando…".into()
        })
        .contains("aria-busy=\"true\""));
    }

    #[test]
    fn variables_incluye_bindings_de_solo_lectura() {
        let arbol = Nodo::Contenedor {
            hijos: vec![Nodo::Variable("contador".into())],
            clase: String::new(),
        };
        assert_eq!(variables(&arbol), vec!["contador"]);
    }

    #[test]
    fn acciones_dentro_de_contenedores_nuevos() {
        let arbol = Nodo::Tarjeta {
            titulo: "x".into(),
            hijos: vec![Nodo::GrupoCampos {
                leyenda: "y".into(),
                hijos: vec![
                    Nodo::Seleccion {
                        nombre: "sel".into(),
                        etiqueta: "S".into(),
                        opciones: vec![],
                        seleccionada: String::new(),
                    },
                    Nodo::Boton {
                        texto: "Ir".into(),
                        accion: "ir".into(),
                        variante: VarianteBoton::Primario,
                        deshabilitado: false,
                    },
                ],
            }],
        };
        assert_eq!(acciones(&arbol), vec!["ir"]);
        assert_eq!(variables(&arbol), vec!["sel"]);
    }

    #[test]
    fn pestanas_aria_y_panel_activo() {
        let nodo = Nodo::Pestanas {
            variable: "pestana".into(),
            items: vec![
                (
                    "lista".into(),
                    "Lista".into(),
                    vec![Nodo::Parrafo("tareas".into())],
                ),
                (
                    "detalle".into(),
                    "Detalle".into(),
                    vec![Nodo::Parrafo("detalle".into())],
                ),
            ],
        };

        // Sin estado: activa la primera.
        let html = render(&nodo);
        assert!(html.contains("role=\"tablist\""));
        assert!(html.contains("aria-selected=\"true\" data-fg-accion=\"tab:pestana:lista\""));
        assert!(html.contains("aria-selected=\"false\" data-fg-accion=\"tab:pestana:detalle\""));
        assert!(html.contains(
            "<div class=\"fg-panel\" role=\"tabpanel\" data-fg-panel=\"lista\" data-fg-panel-var=\"pestana\" aria-label=\"Lista\">"
        ));
        assert!(html.contains("data-fg-panel=\"detalle\" data-fg-panel-var=\"pestana\" aria-label=\"Detalle\" hidden>"));

        // Con estado: cambia la activa y el panel oculto.
        let mut estado = Estado::nuevo();
        estado.fijar("pestana", "detalle");
        let con_estado = render_con_estado(&nodo, &estado);
        assert!(
            con_estado.contains("aria-selected=\"true\" data-fg-accion=\"tab:pestana:detalle\"")
        );
        assert!(con_estado.contains(
            "data-fg-panel=\"lista\" data-fg-panel-var=\"pestana\" aria-label=\"Lista\" hidden>"
        ));
        assert!(con_estado.contains(
            "data-fg-panel=\"detalle\" data-fg-panel-var=\"pestana\" aria-label=\"Detalle\">"
        ));

        // La variable de las pestañas es un binding.
        assert_eq!(variables(&nodo), vec!["pestana"]);

        // Y las de adentro también (regresión: la lista de selección dentro
        // de una pestaña no se sembraba y quedaba sin opción marcada).
        let con_lista = Nodo::Pestanas {
            variable: "pestana".into(),
            items: vec![(
                "uno".into(),
                "Uno".into(),
                vec![Nodo::ListaSeleccion {
                    variable: "vista".into(),
                    etiqueta: "Vista".into(),
                    por_defecto: "hoy".into(),
                    opciones: vec![("hoy".into(), "Hoy".into())],
                }],
            )],
        };
        assert_eq!(variables(&con_lista), vec!["pestana", "vista"]);
    }

    #[test]
    fn plegable_nativo() {
        let nodo = Nodo::Plegable {
            titulo: "Opciones".into(),
            hijos: vec![Nodo::Parrafo("adentro".into())],
            abierto: true,
        };
        let html = render(&nodo);
        assert!(html.starts_with("<details class=\"fg-plegable\" open><summary>Opciones</summary>"));
        assert!(html.ends_with("</details>"));
        assert!(!render(&Nodo::Plegable {
            titulo: "x".into(),
            hijos: vec![],
            abierto: false
        })
        .contains(" open"));
    }

    #[test]
    fn lista_de_seleccion_aria() {
        let nodo = Nodo::ListaSeleccion {
            variable: "vista".into(),
            etiqueta: "Vista".into(),
            por_defecto: "hoy".into(),
            opciones: vec![
                ("hoy".into(), "Hoy".into()),
                ("semana".into(), "Semana".into()),
            ],
        };
        let html = render(&nodo);
        assert!(html.contains("role=\"listbox\""));
        assert!(html.contains("aria-labelledby=\"fg-vista-etiqueta\""));
        assert!(html.contains("data-fg-listbox=\"vista\""));
        // Sin estado, se marca `por_defecto`.
        assert!(html.contains("aria-selected=\"true\" data-fg-accion=\"sel:vista:hoy\""));
        assert!(html.contains("aria-selected=\"false\" data-fg-accion=\"sel:vista:semana\""));

        let mut estado = Estado::nuevo();
        estado.fijar("vista", "semana");
        let con_estado = render_con_estado(&nodo, &estado);
        assert!(con_estado.contains("aria-selected=\"true\" data-fg-accion=\"sel:vista:semana\""));

        assert_eq!(variables(&nodo), vec!["vista"]);
    }

    #[test]
    fn boton_deshabilitado_y_boton_icono() {
        let deshabilitado = render(&Nodo::Boton {
            texto: "Eliminar".into(),
            accion: "eliminar".into(),
            variante: VarianteBoton::Secundario,
            deshabilitado: true,
        });
        assert!(deshabilitado.contains(" disabled>Eliminar</button>"));

        let icono = render(&Nodo::BotonIcono {
            icono: "★".into(),
            etiqueta: "Marcar favorito".into(),
            accion: "favorito".into(),
            deshabilitado: false,
        });
        assert!(icono.contains("aria-label=\"Marcar favorito\""));
        assert!(icono.contains("class=\"fg-boton fg-boton-icono\""));
        assert!(icono.contains("data-fg-accion=\"favorito\""));
        assert!(icono.ends_with(">★</button>"));
        assert!(!icono.contains(" disabled"));
    }

    #[test]
    fn campo_con_error_lo_anuncia() {
        let nodo = Nodo::Entrada {
            nombre: "email".into(),
            tipo: TipoEntrada::Email,
            valor: "no-es-correo".into(),
            marcador: String::new(),
            requerido: true,
            error: Some("Formato inválido <y peligroso>".into()),
        };
        let html = render(&nodo);
        assert!(html.contains("aria-invalid=\"true\""));
        assert!(html.contains("aria-describedby=\"fg-email-error\""));
        assert!(html.contains(
            "<p class=\"fg-error\" id=\"fg-email-error\" role=\"alert\">Formato inválido &lt;y peligroso&gt;</p>"
        ));

        // Sin error no se agrega nada.
        let limpio = render(&Nodo::Entrada {
            nombre: "email".into(),
            tipo: TipoEntrada::Email,
            valor: String::new(),
            marcador: String::new(),
            requerido: false,
            error: None,
        });
        assert!(!limpio.contains("aria-invalid"));
        assert!(!limpio.contains("fg-error"));
    }

    #[test]
    fn tabla_ordenable_con_aria_sort() {
        let nodo = Nodo::Tabla {
            cabeceras: vec!["Tarea".into(), "Estado".into()],
            filas: vec![vec![Nodo::Texto("a".into()), Nodo::Texto("b".into())]],
            orden: Some(("Tarea".into(), Orden::Descendente)),
        };
        let html = render(&nodo);
        assert!(html.contains("<th aria-sort=\"descending\">"));
        assert!(html.contains("data-fg-accion=\"orden:Tarea\""));
        // La otra cabecera está sin ordenar.
        assert!(html.contains("<th aria-sort=\"none\">"));
        assert!(html.contains("data-fg-accion=\"orden:Estado\""));

        // Sin `orden` la tabla no es interactiva.
        let simple = render(&Nodo::Tabla {
            cabeceras: vec!["a".into()],
            filas: vec![],
            orden: None,
        });
        assert!(simple.contains("<th>a</th>"));
        assert!(!simple.contains("aria-sort"));
    }

    #[test]
    fn formulario_real_con_enter_y_post_sin_js() {
        let nodo = Nodo::Formulario {
            accion: "/tareas".into(),
            metodo: "post".into(),
            hijos: vec![Nodo::Entrada {
                nombre: "titulo".into(),
                tipo: TipoEntrada::Texto,
                valor: String::new(),
                marcador: String::new(),
                requerido: true,
                error: None,
            }],
        };
        let html = render(&nodo);
        assert!(html.starts_with(
            "<form class=\"fg-formulario\" method=\"post\" action=\"/tareas\" data-fg-formulario=\"1\">"
        ));
        assert!(html.contains("name=\"titulo\""));
        assert!(html.ends_with("</form>"));

        // Sin acción: el runtime decide (no se pone `action`).
        let sin_accion = render(&Nodo::Formulario {
            accion: String::new(),
            metodo: String::new(),
            hijos: vec![],
        });
        assert!(!sin_accion.contains("action="));
        assert!(sin_accion.contains("method=\"post\""));
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
                        deshabilitado: false,
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
        Nodo::Contenedor { hijos, .. }
        | Nodo::Lista { items: hijos, .. }
        | Nodo::GrupoCampos { hijos, .. }
        | Nodo::Tarjeta { hijos, .. }
        | Nodo::Dialogo { hijos, .. }
        | Nodo::Plegable { hijos, .. }
        | Nodo::Formulario { hijos, .. } => {
            for hijo in hijos {
                recolectar_acciones(hijo, salida);
            }
        }
        Nodo::Pestanas { items, .. } => {
            for (_, _, hijos) in items {
                for hijo in hijos {
                    recolectar_acciones(hijo, salida);
                }
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
        // Una variable de solo lectura también es un binding: su valor inicial
        // sale del programa, igual que el de un input.
        Nodo::Variable(nombre) => {
            if !salida.contains(nombre) {
                salida.push(nombre.clone());
            }
        }
        Nodo::Entrada { nombre, .. }
        | Nodo::Casilla { nombre, .. }
        | Nodo::Seleccion { nombre, .. }
        | Nodo::AreaTexto { nombre, .. } => {
            if !salida.contains(nombre) {
                salida.push(nombre.clone());
            }
        }
        Nodo::Contenedor { hijos, .. }
        | Nodo::Lista { items: hijos, .. }
        | Nodo::GrupoCampos { hijos, .. }
        | Nodo::Tarjeta { hijos, .. }
        | Nodo::Dialogo { hijos, .. }
        | Nodo::Plegable { hijos, .. }
        | Nodo::Formulario { hijos, .. } => {
            for hijo in hijos {
                recolectar_variables(hijo, salida);
            }
        }
        Nodo::ListaSeleccion { variable, .. } => {
            if !salida.contains(variable) {
                salida.push(variable.clone());
            }
        }
        Nodo::Pestanas { variable, items } => {
            if !salida.contains(variable) {
                salida.push(variable.clone());
            }
            for (_, _, hijos) in items {
                for hijo in hijos {
                    recolectar_variables(hijo, salida);
                }
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
