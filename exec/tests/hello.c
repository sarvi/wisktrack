#include <stdio.h>
#include "sum.h"
#include "difference.h"
#include "quotient.h"
#include "square.h"

int main()
{
   printf("Hello World\n");
   
   int a = 10, b = 5;
   
   printf("Adding %d and %d = %d\n", a, b, add(a,b));
   printf("Subtracting %d and %d = %d\n", a, b, subtract(a,b));
   printf("Multiplying %d and %d = %d\n", a, b, multiply(a,b));
   printf("Dividing %d and %d = %d\n", a, b, divide(a,b));
   printf("Square of %d = %d\n", a, squared(a));
   printf("Square of %d = %d\n", b, squared(b));

   return 0;
}
