# stereogram-webgl
Un autoestereograma (también conocido como Magic Eye) es una imagen 2D hecha para crear la ilusión de 3D. En cada imagen hay un objeto 3D que solo se ve si mirás la imagen de cierta manera, como si la pantalla fuera transparente y estuvieras mirando la pared de atrás. Con la práctica se vuelve más fácil.

Los autoestereogramas fueron muy populares en los años 90. Aprovechan la estereopsis: el cerebro reconstruye la profundidad combinando las dos imágenes levemente distintas que percibe cada ojo.

En este proyecto podés usar tu propio mapa de profundidad, personalizar los patrones y la forma en que se calcula la imagen. Todo corre en la GPU y también puede mostrar escenas en movimiento en tiempo real.

Miralo en vivo [acá](https://pablogventura.github.io/stereogram-webgl/).

Mirá el solver de Magic Eye [acá](https://piellardj.github.io/stereogram-solver/).

[![Donar](https://raw.githubusercontent.com/piellardj/piellardj.github.io/master/images/readme/donate-paypal.svg)](https://www.paypal.com/donate/?hosted_button_id=AF7H7GEJTL95E)

## Vista previa

![Planeta](src/readme/preview_planet.jpg)

![Barco](src/readme/preview_ship.jpg)

![Onda](src/readme/preview_ripple.jpg)

## Cómo ver un estereograma
### Instrucciones
Ver la escena 3D escondida en un estereograma pide un poco de práctica. La clave es mirar más allá de la imagen, sin enfocar la imagen en sí. Hay varias formas de hacerlo:
- si podés, relajá los ojos a propósito, como si estuvieras mirando a lo lejos
- otra técnica es empezar con la cabeza muy cerca de la pantalla, tan cerca que no puedas enfocarla. En ese punto los ojos deberían mirar a lo lejos de forma natural. Después alejás la cabeza despacio, sin mirar la pantalla: tratá de seguir mirando al vacío
- otra es usar un objeto, como una lapicera: ponela detrás de la pantalla, de costado, para poder verla. Seguí mirándola y acercala despacio hacia el centro de la pantalla. Eso ayuda a mirar más allá de la pantalla.

Si no llegás a ver un objeto 3D nítido pero igual sentís algo raro, vas por buen camino.

### Ejercicios
Estas imágenes sirven para practicar. En cada una hay que mirar de la forma correcta.

<div style="text-align:center;margin:48px 0">
    <p>
        <i>Relajá los ojos y deberías ver aparecer un tercer punto negro en el medio.</i>
    </p>
    <img alt="Primera imagen de práctica" src="src/readme/tutorial_easy.png"/>
</div>

<div style="text-align:center;margin:48px 0">
    <p>
        <i>Practicá la mirada con esta imagen. Cada fila debería flotar a una profundidad distinta: las margaritas se ven lejos, las flores rojas un poco más cerca, después la mariposa azul, y las flores amarillas son las más cercanas.</i>
    </p>
    <img alt="Segunda imagen de práctica" src="src/readme/tutorial_medium.png"/>
</div>

<div style="text-align:center;margin:48px 0">
    <p>
        <i>Esta es la última imagen de práctica. Si la mirás bien, deberías ver con claridad un objeto 3D.</i>
    </p>
    <img alt="Última imagen de práctica" src="src/readme/tutorial_hard.jpg"/>
</div>

## Cómo funciona
### Idea de base

El cerebro percibe la profundidad combinando las dos imágenes levemente distintas que llegan de los ojos. Si un objeto se ve igual desde los dos ojos, está lejos. Si se ve muy distinto desde cada ojo, cada ojo tiene una perspectiva diferente, así que el objeto está cerca.

Como la percepción de profundidad se basa en la visión binocular, para mostrar un objeto 3D en una imagen 2D hay que hacerle creer al cerebro que cada ojo ve una perspectiva distinta. Si mirás la imagen de frente, no funciona: el cerebro ve con claridad que es 2D. Por eso hay que enfocar un punto delante de la imagen, o detrás.

Para construir la imagen, veamos qué pasa si alguien mira un objeto a través de una pantalla transparente:

<div style="text-align:center">
    <img alt="La profundidad crea una repetición" src="src/readme/diagram-01.png"/>
    <p>
        <i>Hay dos "fantasmas" del objeto sobre la pantalla, porque los ojos están en posiciones distintas respecto del objeto.</i>
    </p>
</div>

Comparemos dos situaciones: un objeto cerca y otro lejos:

<div style="text-align:center">
    <img alt="El período de repetición cambia con la distancia" src="src/readme/diagram-02.png"/>
    <p>
        <i>Cuanto más cerca está el objeto de la pantalla, más cerca quedan sus fantasmas.</i>
    </p>
</div>

En conclusión, si no mirás la imagen de frente sino más allá, al repetir un patrón el cerebro lo interpreta como si ese patrón estuviera detrás de la pantalla. Además, al cambiar la frecuencia de las repeticiones se puede dar una impresión de profundidad relativa: un patrón con repeticiones cercanas parece más cerca que uno con repeticiones más espaciadas. Por eso, en la imagen de abajo, las margaritas parecen más lejos que las mariposas.

<div style="text-align:center">
    <img alt="Percepción de profundidad y frecuencia de repetición" src="src/readme/tutorial_medium.png"/>
    <p>
        <i>Al mirar más allá de esta imagen, las margaritas parecen más lejos que las mariposas por la diferencia en la frecuencia de repetición.</i>
    </p>
</div>

Otra propiedad útil es que alcanza con una sola repetición para percibir profundidad. En una misma fila se puede variar la profundidad percibida cambiando la frecuencia de las repeticiones, y eso crea un efecto de pendiente.

<div style="text-align:center">
    <img alt="Se puede cambiar la profundidad en una sola fila" src="src/readme/frequency-change.png"/>
    <p>
        <i>En cada fila, la frecuencia de repetición varía. Eso da la percepción de un cambio de profundidad a lo largo de la fila.</i>
    </p>
</div>

### Algoritmo
Queremos generar un autoestereograma a partir de:
- un mapa de profundidad, una imagen en blanco y negro, en este ejemplo una esfera;
- un patrón de base, en este ejemplo una textura continua de nubes.

Como todo se trata de repetición, definimos un período de repetición en píxeles. Después partimos la imagen en franjas verticales, cada una con ese período de ancho.

<div style="text-align:center">
    <img alt="Imagen partida en franjas" src="src/readme/demo_stripes_empty.png"/>
    <p>
        <i>Imagen partida en 5 franjas verticales, más la franja de referencia a la izquierda.</i>
    </p>
</div>

Hacemos que cada franja sea una repetición de la anterior. La franja de la izquierda sirve de referencia:

<div style="text-align:center">
    <img alt="Repetición sin desplazamiento" src="src/readme/demo_stripes.png"/>
    <p>
        <i>Al repetir la primera franja sin desplazamiento, solo se percibe una superficie plana.</i>
    </p>
</div>

Para que sea 3D, agregamos un desplazamiento horizontal: cada franja es una versión desplazada de la anterior. Cada píxel tiene su propio desplazamiento, tomado del mapa de profundidad en esa posición. La franja de la izquierda es la primera, así que no tiene referencia y no se deforma. Para no recortar el mapa de profundidad, se lo achica y se lo corre a la derecha, de modo que se use completo.

<div style="text-align:center">
    <img alt="Repetición con desplazamiento" src="src/readme/demo_final.png"/>
    <p>
        <i>Al repetir la primera franja y sumarle un desplazamiento, se perciben los objetos 3D.</i>
    </p>
</div>

Al final, cada píxel de la imagen es una versión desplazada de la franja de origen. Esta ilustración muestra mejor el desplazamiento:

<div style="text-align:center">
    <img alt="Repetición con desplazamiento" src="src/readme/demo_uv.png"/>
    <p>
        <i>Al mostrar las coordenadas UV en las que se muestrea el patrón, el desplazamiento se ve con más claridad. Se distingue la forma de esfera tomada del mapa de profundidad.</i>
    </p>
</div>
