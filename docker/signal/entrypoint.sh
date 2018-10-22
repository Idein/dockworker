#!/bin/sh

trap 'echo HUP              ' HUP
trap 'echo INT              ' INT
trap 'echo QUIT             ' QUIT
trap 'echo ILL              ' ILL
trap 'echo TRAP             ' TRAP

trap 'echo ABRT             ' ABRT
trap 'echo BUS              ' BUS
trap 'echo FPE              ' FPE
trap 'echo KILL    ; exit  9' KILL
trap 'echo USR1             ' USR1

trap 'echo SEGV             ' SEGV
trap 'echo USR2             ' USR2
trap 'echo PIPE             ' PIPE
trap 'echo ALRM             ' ALRM
trap 'echo TERM    ; exit 15' TERM

#trap 'echo STKFLT  ; exit 16' STKFLT
trap 'echo CHLD             ' CHLD
trap 'echo CONT             ' CONT
trap 'echo STOP    ; exit 19' STOP
trap 'echo TSTP             ' TSTP

trap 'echo TTIN             ' TTIN
trap 'echo TTOU             ' TTOU
trap 'echo URG              ' URG
trap 'echo XCPU             ' XCPU
trap 'echo XFSZ             ' XFSZ

trap 'echo VTALRM           ' VTALRM
trap 'echo PROF             ' PROF
trap 'echo WINCH            ' WINCH
trap 'echo IO               ' IO
trap 'echo PWR              ' PWR

trap 'echo SYS              ' SYS
trap 'echo RTMIN            ' RTMIN
trap 'echo RTMIN+1          ' RTMIN+1
trap 'echo RTMIN+2          ' RTMIN+2
trap 'echo RTMIN+3          ' RTMIN+3

trap 'echo RTMIN+4          ' RTMIN+4
trap 'echo RTMIN+5          ' RTMIN+5
trap 'echo RTMIN+6          ' RTMIN+6
trap 'echo RTMIN+7          ' RTMIN+7
trap 'echo RTMIN+8          ' RTMIN+8

trap 'echo RTMIN+9          ' RTMIN+9
trap 'echo RTMIN+10         ' RTMIN+10
trap 'echo RTMIN+11         ' RTMIN+11
trap 'echo RTMIN+12         ' RTMIN+12
trap 'echo RTMIN+13         ' RTMIN+13

trap 'echo RTMIN+14         ' RTMIN+14
trap 'echo RTMIN+15         ' RTMIN+15
trap 'echo RTMAX-14         ' RTMAX-14
trap 'echo RTMAX-13         ' RTMAX-13
trap 'echo RTMAX-12         ' RTMAX-12

trap 'echo RTMAX-11         ' RTMAX-11
trap 'echo RTMAX-10         ' RTMAX-10
trap 'echo RTMAX-9          ' RTMAX-9
trap 'echo RTMAX-8          ' RTMAX-8
trap 'echo RTMAX-7          ' RTMAX-7

trap 'echo RTMAX-6          ' RTMAX-6
trap 'echo RTMAX-5          ' RTMAX-5
trap 'echo RTMAX-4          ' RTMAX-4
trap 'echo RTMAX-3          ' RTMAX-3
trap 'echo RTMAX-2          ' RTMAX-2

trap 'echo RTMAX-1          ' RTMAX-1
trap 'echo RTMAX            ' RTMAX

while :; do :; done

