# Sistema Solar - Proyecto Shader-Based 🚀🪐

Sistema solar completo renderizado con **software rasterizer** y **shaders procedurales**

![Demostración del Sistema Solar](image.png)
*Vista general del sistema solar completo*

![Vista General de Planetas](general.png)
*Diferentes planetas del sistema con sus características únicas*

---

## 📝 Descripción del Proyecto

Sistema solar interactivo con 8 cuerpos celestes únicos, todos generados mediante **shaders procedurales avanzados** usando técnicas de ruido (FBM, Turbulence, Worley). Implementado en **Rust** con rasterización por software y optimizaciones multi-core.

### 🎮 Controles
- **W / S**: Subir o bajar la cámara
- **A / D**: Desplazamiento lateral (strafe)
- **← / →**: Orbitar la cámara alrededor del objetivo
- **↑ / ↓**: Avanzar o retroceder manteniendo la orientación
- **Z / X**: Zoom In/Out sobre el punto de interés
- **O**: Mostrar u ocultar las órbitas
- **ESC**: Salir del programa

---

[![Video demostrativo](assets/Estetica.png)](https://youtu.be/jQ9-BoPCNgo)


### Estética General
Inspirada en un sistema solar alienígena: shaders con bioluminiscencia, niebla y bandas cromáticas crean una escena coherente y llamativa.

![Captura estética principal](assets/Estetica.png)

### Performance de la Escena
El rasterizador por software aprovecha paralelismo con `rayon`; aunque la tasa de cuadros fluctúa, se mantiene utilizable incluso en órbitas densas.

[![Indicadores de performance](assets/Estetica.png)](https://youtu.be/EhYAIewqwuw)

### Planetas, Estrellas y Lunas
Se incluyen Sol, Tierra+Luna, Marte, Saturno con anillos, Ice Planet y Alien Planet; todos con shaders procedurales distintos.

![Galería de cuerpos celestes](assets/planetas.png)

### Nave Personalizada
El Airwing inspirado en StarFox fue modelado en Blender, se renderiza con el mismo rasterizador y mantiene animaciones de alabeo ligadas a la cámara.

![Airwing persiguiendo la cámara](assets/airwing.png)
![Airwing persiguiendo la cámara](assets/image.png)


### Skybox Estelar
Un campo estelar procedural envuelve la escena para dar profundidad y referencia visual en el horizonte.

![Skybox estelar](assets/skybox.png)

### Colisiones Nave/Cámara
El detector de colisiones calcula penetración contra cada cuerpo (excepto el sol) y evita que la nave atraviese planetas o lunas.

![Demostración de colisión](assets/colisiones.png)

### Movimiento 3D de Cámara
La cámara admite traslación 3D, órbitas, zoom relativo y control desacoplado para navegar libremente por el sistema.

[![Trayectoria de cámara](assets/Estetica.png)](https://youtu.be/EhYAIewqwuw)

### Órbitas Renderizadas
Cada planeta y la Luna muestran su órbita elíptica con segmentación adaptativa, facilitando entender sus trayectorias.

[![Órbitas visibles](assets/Estetica.png)](assets/skybox.png)



