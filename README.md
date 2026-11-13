# forja-gui-dom

Modelo de nodos **DOM** para interfaces Forja, con renderizado a HTML
(escape automático y atributos de accesibilidad) y estado/eventos para
actualizaciones reactivas.

Es el camino elegido en el plan de Forja para reemplazar el `<canvas>` de la GUI
web: el mismo árbol de interfaz se pinta como elementos reales en el navegador y
como HTML en el servidor (SSR).

## Demo

**https://forja-lang.github.io/forja-gui-dom/** — interfaz Forja compilada a
wasm pintando DOM real: los clics ejecutan callbacks del lenguaje y escribir en
un campo actualiza la variable del programa.

## Piezas

| Pieza | Qué hace |
|---|---|
| `Nodo` | árbol de interfaz: contenedor, texto, variable, encabezado, párrafo, enlace, botón con acción declarativa, entrada, casilla, selección, área de texto, grupo de campos, progreso, insignia, tarjeta, aviso, navegación, diálogo, imagen, carga, separador |
| `render` / `documento` | HTML escapado; `documento` arma la página completa para SSR |
| `Estado` | variables de la interfaz (`fijar`, `obtener`, `booleano`) |
| `Evento` + `aplicar` | `Click`, `Cambio`, `Marcar` → actualiza el estado |
| `render_con_estado` | re-render con los valores actuales |
| `acciones` / `variables` | listas para enganchar delegación de eventos y bindings sin recorrer el DOM |
| `eventos::ACCION` | `data-fg-accion`: la acción que el runtime mapea a una función Forja |

Todo lo que emite usa clases `fg-*`, definidas en [`estilos.css`](estilos.css)
(hay un test que falla si se emite una clase sin estilo).

## Tests

```bash
cargo test -p forja-gui-dom
```

21 tests unitarios y 3 de integración (contrato con `estilos.css`, página
completa). La verificación en navegador de la demo vive en el repo de Forja
(`scripts/probar-demo-dom.sh`, Chrome/Edge headless).
