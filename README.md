### Descripción del Diseño

El `BooleanField` es una solución optimizada para la gestión de campos booleanos en bases de datos. A diferencia de las implementaciones estándar que suelen asignar un byte completo para un valor booleano (`true`, `false`, `NULL`), este diseño utiliza una codificación de bits para empaquetar de manera eficiente el valor, las restricciones de integridad y las propiedades por defecto en un solo byte.

El diseño se basa en un conjunto de **13 estados válidos** que representan todas las combinaciones posibles de valor (`true`, `false`, `NULL`), restricción (`NOT NULL`) y valor por defecto. La estructura de datos garantiza que cualquier valor fuera de estas combinaciones sea considerado inválido, lo que refuerza la integridad del dato a nivel fundamental.

-----

### Principios de Ingeniería

Este `BooleanField` destaca por aplicar principios de diseño robustos y modernos:

#### 1\. Eficiencia de Memoria y Empaquetamiento de Bits

El núcleo del diseño reside en la estructura `PackedBooleanData`, que utiliza un tipo primitivo `u8` (8 bits) para almacenar toda la información del campo. Esto elimina el desperdicio de los 7 bits que a menudo no se utilizan en implementaciones tradicionales. El espacio ganado no solo se usa para el valor (`true`, `false`), sino también para codificar el estado `NULL` y las propiedades `NOT NULL` y `default`.

#### 2\. Integridad por Diseño: El ADN Autoconsciente del Dato

La validación no es un proceso externo; es una propiedad inherente al dato. Las funciones `encode_state` y `decode_state` actúan como guardianes, permitiendo solo que los 13 estados válidos sean representados. Esto asegura que los datos sean siempre coherentes y evita la corrupción desde la capa de entrada. Si un valor inválido se introduce en la base de datos, la función de decodificación lo detectará y rechazará al intentar leerlo.

El campo es "autoconsciente" porque **lleva consigo su propia definición de lo que es válido**. Su "ADN" es la firma de bits que determina su identidad y garantiza que su valor nunca pueda contradecir su propia naturaleza. La validación se convierte en un acto intrínseco de lectura y escritura.

#### 3\. Separación de Responsabilidades

El código sigue un patrón de diseño limpio y modular. Las responsabilidades están claramente separadas:

  * `PackedBooleanData` y `BooleanOps` se encargan de la lógica de bajo nivel, las operaciones de empaquetamiento de bits y las operaciones lógicas, optimizadas para el rendimiento.
  * `BooleanField` es una interfaz de alto nivel que ofrece una API intuitiva y legible para el desarrollador, ocultando la complejidad subyacente. Esta separación facilita la lectura, el mantenimiento y la extensibilidad del código.

-----

### Uso y Funcionalidad

El tipo de dato `BooleanField` es versátil y fácil de usar, permitiendo la creación de campos con diversas propiedades:

```rust
// Crear un campo con restricción NOT NULL y valor por defecto
let field = BooleanField::<&str>::new()
    .not_null()
    .default(true);

// El campo autovalida el valor
let mut field_with_value = field;
assert!(field_with_value.set_value(None).is_err()); // Intenta asignar NULL, lo cual falla.

// El campo genera su definición SQL de forma dinámica
println!("{}", field.to_sql());
// Salida: BOOLEAN NOT NULL DEFAULT TRUE
```

Este diseño no solo aborda la eficiencia de la memoria, sino que también ofrece un modelo robusto y seguro para la gestión de datos booleanos, lo que lo convierte en un componente valioso para cualquier biblioteca de base de datos o framework.