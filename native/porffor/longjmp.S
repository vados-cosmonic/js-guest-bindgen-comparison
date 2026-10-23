# One shared exception tag for Porffor and WASI SDK's libsetjmp.so.
.globl __c_longjmp
.tagtype __c_longjmp i32
__c_longjmp:
