struct IdentifierName {
    name: String,
}

enum Quantity {
    Const(u128),
    Generic(IdentifierName),
}

enum Type {
    Block(Quantity),
    Pointer(Box<Type>),
}

struct FunctionInput {
    var: IdentifierName,
    ty: Type,
}
struct FunctionDefinition {
    name: IdentifierName,
    generic_quantities: Vec<IdentifierName>,
    inputs: Vec<FunctionInput>,
    output_types: Vec<Type>,
    body: Vec<Statement>,
}

struct Word {}

enum PrimitiveFunction {
    Not(Box<Expression>),
    Add(Quantity, Box<Expression>, Box<Expression>),
}

enum Expression {
    Constant(Word),
    Variable(IdentifierName),
    DeRef(Box<Expression>),
    Ref(IdentifierName),
    Function {
        func: IdentifierName,
        generic_quantities: Vec<Quantity>,
        inputs: Vec<Expression>,
    },
    PrimitiveFunction(PrimitiveFunction),
}

enum Statement {
    VarType {
        var: IdentifierName,
        ty: Type,
    },
    VarAssign {
        vars: Vec<IdentifierName>,
        expression: Expression,
    },
    If {
        condition: Expression,
        body: Vec<Statement>,
    },
}

fn main() {
    println!("Hello, world!");
}

/*

// Pseudo primitive functions
Function 1 New(num:1) {
    // Allocate <num> words on the heap and return a pointer to the first word
}

Function Del(addr : 1) {
    // Delete the heap entry starting at <addr>
}


Function 1,1 bar(a, b) {
    return a + b, a - b;
}
{
    let a = 1;
    let b = 2;
    let c, d = foo(a, b);
}

// a and b both point to N-word values
Function<n> n Add(a : &n, b : &n) {

}

{
    Let a = 1, 2, 3;
    Let b = 4, 5, 6;

    MultiAddPointers<3>(&a, &b);
}

Function 2 add_with_carry(a, b) {
    // *a:2
    // *b:2



    Let c = (*a+0)+(*b+0);

    (*a+0) & (1 << (N-1))

    (*a+0) & 1

    (*b+0) & 1

    Let d = (*b+1)+(*a+1)
}


Function 0 main() {
    Let a : 1 = x; // Allocate 1 word on the stack for a and write x to it
    // Same as
    Let a : 1; // Allocate 1 word on the stack for a
    Let a = x; // Write x to the location on the stack for a

    Let b : 3 = 12, 13, 14;
    // b -> 12
    // b + 1 -> 13
    // b + 2 -> 14
    // b + 3 -> ? // Valid
    // Can have the compiler check these:
    // b[0] -> 12
    // b[1] -> 13
    // b[2] -> 14
    // b[3] -> ? // Invalid

    Let _, _, _, _ = New(&3, &&4, 5);

    Let c = New(5);
    // Implicitly c:1
    Let d = *c; // d:1 is set to the thing in RAM pointed at by c
    Let d = *c[0];
    Let e = *(c+1); // Invalid since c:1    Need c+1
    Del(c); // Free the heap memory at c

    // Is x non-zero?
    If x {
        // Then do stuff
    }

    // Loop while x is non-zero
    While x {
        // Do stuff
    }
}

*/
