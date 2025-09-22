const PI: f32 = 3.141592653589793;      // π
const PI_2: f32 = 6.283185307179586;    // 2π
const HALF_PI: f32 = 1.57079632679;     // π/2
const FRAC_PI_3: f32 = 1.0471975512;    // π/3
const E: f32 = 2.718281828459045;       // exp(1)

const MY_PI: f32 = PI;
const MY_PI_PARENTHESES: f32 = (((PI)));

alias IVec3 = vec3<i32>;
alias SomeType = vec3<IVec3, f32, vec2<i32>>;
alias SomeArray = array<vec3<f32>, 4>;
alias SomeDynamicArray = array<vec3<f32>>;
