; Comparison-only DOS COM. Build: nasm -f bin reference_dos_path.asm -o PROBE.COM
; Mount original GOG app as C: read-only and a disposable output directory as D:.
; Run PROBE.COM on D:. READ.BIN must match the first32 bytes of MM_SLIDE.GXA.
bits 16
org 0x100
mov dx,path
mov ax,0x3d00
int 0x21
jc failed
mov bx,ax
mov dx,buffer
mov cx,32
mov ah,0x3f
int 0x21
jc failed
cmp ax,32
jne failed
mov ah,0x3e
int 0x21
mov dx,output
xor cx,cx
mov ah,0x3c
int 0x21
jc failed
mov bx,ax
mov dx,buffer
mov cx,32
mov ah,0x40
int 0x21
jc failed
cmp ax,32
jne failed
mov ah,0x3e
int 0x21
mov ax,0x4c00
int 0x21
failed:
mov ax,0x4c02
int 0x21
path db 'C:\Redguard\system\mm_slider.gxa',0
output db 'D:\READ.BIN',0
buffer times 32 db 0
