
kernel:     file format elf64-littleriscv


Disassembly of section .text:

0000000080000000 <_entry>:
    80000000:	00008117          	auipc	sp,0x8
    80000004:	86010113          	add	sp,sp,-1952 # 80007860 <stack0>
    80000008:	6505                	lui	a0,0x1
    8000000a:	f14025f3          	csrr	a1,mhartid
    8000000e:	0585                	add	a1,a1,1
    80000010:	02b50533          	mul	a0,a0,a1
    80000014:	912a                	add	sp,sp,a0
    80000016:	04a000ef          	jal	80000060 <start>

000000008000001a <spin>:
    8000001a:	a001                	j	8000001a <spin>

000000008000001c <timerinit>:
    8000001c:	1141                	add	sp,sp,-16
    8000001e:	e422                	sd	s0,8(sp)
    80000020:	0800                	add	s0,sp,16
    80000022:	304027f3          	csrr	a5,mie
    80000026:	0207e793          	or	a5,a5,32
    8000002a:	30479073          	csrw	mie,a5
    8000002e:	30a027f3          	csrr	a5,menvcfg
    80000032:	577d                	li	a4,-1
    80000034:	177e                	sll	a4,a4,0x3f
    80000036:	8fd9                	or	a5,a5,a4
    80000038:	30a79073          	csrw	menvcfg,a5
    8000003c:	306027f3          	csrr	a5,mcounteren
    80000040:	0027e793          	or	a5,a5,2
    80000044:	30679073          	csrw	mcounteren,a5
    80000048:	c01027f3          	rdtime	a5
    8000004c:	000f4737          	lui	a4,0xf4
    80000050:	24070713          	add	a4,a4,576 # f4240 <_entry-0x7ff0bdc0>
    80000054:	97ba                	add	a5,a5,a4
    80000056:	14d79073          	csrw	stimecmp,a5
    8000005a:	6422                	ld	s0,8(sp)
    8000005c:	0141                	add	sp,sp,16
    8000005e:	8082                	ret

0000000080000060 <start>:
    80000060:	1141                	add	sp,sp,-16
    80000062:	e406                	sd	ra,8(sp)
    80000064:	e022                	sd	s0,0(sp)
    80000066:	0800                	add	s0,sp,16
    80000068:	300027f3          	csrr	a5,mstatus
    8000006c:	7779                	lui	a4,0xffffe
    8000006e:	7ff70713          	add	a4,a4,2047 # ffffffffffffe7ff <end+0xffffffff7ffddc97>
    80000072:	8ff9                	and	a5,a5,a4
    80000074:	6705                	lui	a4,0x1
    80000076:	80070713          	add	a4,a4,-2048 # 800 <_entry-0x7ffff800>
    8000007a:	8fd9                	or	a5,a5,a4
    8000007c:	30079073          	csrw	mstatus,a5
    80000080:	00001797          	auipc	a5,0x1
    80000084:	dd678793          	add	a5,a5,-554 # 80000e56 <main>
    80000088:	34179073          	csrw	mepc,a5
    8000008c:	4781                	li	a5,0
    8000008e:	18079073          	csrw	satp,a5
    80000092:	67c1                	lui	a5,0x10
    80000094:	17fd                	add	a5,a5,-1 # ffff <_entry-0x7fff0001>
    80000096:	30279073          	csrw	medeleg,a5
    8000009a:	30379073          	csrw	mideleg,a5
    8000009e:	104027f3          	csrr	a5,sie
    800000a2:	2207e793          	or	a5,a5,544
    800000a6:	10479073          	csrw	sie,a5
    800000aa:	57fd                	li	a5,-1
    800000ac:	83a9                	srl	a5,a5,0xa
    800000ae:	3b079073          	csrw	pmpaddr0,a5
    800000b2:	47bd                	li	a5,15
    800000b4:	3a079073          	csrw	pmpcfg0,a5
    800000b8:	f65ff0ef          	jal	8000001c <timerinit>
    800000bc:	f14027f3          	csrr	a5,mhartid
    800000c0:	2781                	sext.w	a5,a5
    800000c2:	823e                	mv	tp,a5
    800000c4:	30200073          	mret
    800000c8:	60a2                	ld	ra,8(sp)
    800000ca:	6402                	ld	s0,0(sp)
    800000cc:	0141                	add	sp,sp,16
    800000ce:	8082                	ret

00000000800000d0 <consolewrite>:
    800000d0:	7159                	add	sp,sp,-112
    800000d2:	f486                	sd	ra,104(sp)
    800000d4:	f0a2                	sd	s0,96(sp)
    800000d6:	eca6                	sd	s1,88(sp)
    800000d8:	e8ca                	sd	s2,80(sp)
    800000da:	e4ce                	sd	s3,72(sp)
    800000dc:	e0d2                	sd	s4,64(sp)
    800000de:	fc56                	sd	s5,56(sp)
    800000e0:	f85a                	sd	s6,48(sp)
    800000e2:	f45e                	sd	s7,40(sp)
    800000e4:	f062                	sd	s8,32(sp)
    800000e6:	1880                	add	s0,sp,112
    800000e8:	04c05463          	blez	a2,80000130 <consolewrite+0x60>
    800000ec:	89b2                	mv	s3,a2
    800000ee:	8aae                	mv	s5,a1
    800000f0:	8a2a                	mv	s4,a0
    800000f2:	4901                	li	s2,0
    800000f4:	4bfd                	li	s7,31
    800000f6:	02000c13          	li	s8,32
    800000fa:	5b7d                	li	s6,-1
    800000fc:	a025                	j	80000124 <consolewrite+0x54>
    800000fe:	86a6                	mv	a3,s1
    80000100:	01590633          	add	a2,s2,s5
    80000104:	85d2                	mv	a1,s4
    80000106:	f9040513          	add	a0,s0,-112
    8000010a:	132020ef          	jal	8000223c <either_copyin>
    8000010e:	03650263          	beq	a0,s6,80000132 <consolewrite+0x62>
    80000112:	85a6                	mv	a1,s1
    80000114:	f9040513          	add	a0,s0,-112
    80000118:	756000ef          	jal	8000086e <uartwrite>
    8000011c:	0124893b          	addw	s2,s1,s2
    80000120:	01395963          	bge	s2,s3,80000132 <consolewrite+0x62>
    80000124:	412984bb          	subw	s1,s3,s2
    80000128:	fc9bdbe3          	bge	s7,s1,800000fe <consolewrite+0x2e>
    8000012c:	84e2                	mv	s1,s8
    8000012e:	bfc1                	j	800000fe <consolewrite+0x2e>
    80000130:	4901                	li	s2,0
    80000132:	854a                	mv	a0,s2
    80000134:	70a6                	ld	ra,104(sp)
    80000136:	7406                	ld	s0,96(sp)
    80000138:	64e6                	ld	s1,88(sp)
    8000013a:	6946                	ld	s2,80(sp)
    8000013c:	69a6                	ld	s3,72(sp)
    8000013e:	6a06                	ld	s4,64(sp)
    80000140:	7ae2                	ld	s5,56(sp)
    80000142:	7b42                	ld	s6,48(sp)
    80000144:	7ba2                	ld	s7,40(sp)
    80000146:	7c02                	ld	s8,32(sp)
    80000148:	6165                	add	sp,sp,112
    8000014a:	8082                	ret

000000008000014c <consoleread>:
    8000014c:	7119                	add	sp,sp,-128
    8000014e:	fc86                	sd	ra,120(sp)
    80000150:	f8a2                	sd	s0,112(sp)
    80000152:	f4a6                	sd	s1,104(sp)
    80000154:	f0ca                	sd	s2,96(sp)
    80000156:	ecce                	sd	s3,88(sp)
    80000158:	e8d2                	sd	s4,80(sp)
    8000015a:	e4d6                	sd	s5,72(sp)
    8000015c:	e0da                	sd	s6,64(sp)
    8000015e:	fc5e                	sd	s7,56(sp)
    80000160:	f862                	sd	s8,48(sp)
    80000162:	f466                	sd	s9,40(sp)
    80000164:	f06a                	sd	s10,32(sp)
    80000166:	ec6e                	sd	s11,24(sp)
    80000168:	0100                	add	s0,sp,128
    8000016a:	8caa                	mv	s9,a0
    8000016c:	8aae                	mv	s5,a1
    8000016e:	8a32                	mv	s4,a2
    80000170:	00060b1b          	sext.w	s6,a2
    80000174:	0000f517          	auipc	a0,0xf
    80000178:	6ec50513          	add	a0,a0,1772 # 8000f860 <cons>
    8000017c:	23b000ef          	jal	80000bb6 <acquire>
    80000180:	09405163          	blez	s4,80000202 <consoleread+0xb6>
    80000184:	0000f497          	auipc	s1,0xf
    80000188:	6dc48493          	add	s1,s1,1756 # 8000f860 <cons>
    8000018c:	89a6                	mv	s3,s1
    8000018e:	0000f917          	auipc	s2,0xf
    80000192:	76a90913          	add	s2,s2,1898 # 8000f8f8 <cons+0x98>
    80000196:	4c11                	li	s8,4
    80000198:	5d7d                	li	s10,-1
    8000019a:	4da9                	li	s11,10
    8000019c:	0984a783          	lw	a5,152(s1)
    800001a0:	09c4a703          	lw	a4,156(s1)
    800001a4:	02f71163          	bne	a4,a5,800001c6 <consoleread+0x7a>
    800001a8:	6e4010ef          	jal	8000188c <myproc>
    800001ac:	723010ef          	jal	800020ce <killed>
    800001b0:	e135                	bnez	a0,80000214 <consoleread+0xc8>
    800001b2:	85ce                	mv	a1,s3
    800001b4:	854a                	mv	a0,s2
    800001b6:	4df010ef          	jal	80001e94 <sleep>
    800001ba:	0984a783          	lw	a5,152(s1)
    800001be:	09c4a703          	lw	a4,156(s1)
    800001c2:	fef703e3          	beq	a4,a5,800001a8 <consoleread+0x5c>
    800001c6:	0017871b          	addw	a4,a5,1
    800001ca:	08e4ac23          	sw	a4,152(s1)
    800001ce:	07f7f713          	and	a4,a5,127
    800001d2:	9726                	add	a4,a4,s1
    800001d4:	01874703          	lbu	a4,24(a4)
    800001d8:	00070b9b          	sext.w	s7,a4
    800001dc:	078b8263          	beq	s7,s8,80000240 <consoleread+0xf4>
    800001e0:	f8e407a3          	sb	a4,-113(s0)
    800001e4:	4685                	li	a3,1
    800001e6:	f8f40613          	add	a2,s0,-113
    800001ea:	85d6                	mv	a1,s5
    800001ec:	8566                	mv	a0,s9
    800001ee:	004020ef          	jal	800021f2 <either_copyout>
    800001f2:	01a50863          	beq	a0,s10,80000202 <consoleread+0xb6>
    800001f6:	0a85                	add	s5,s5,1
    800001f8:	3a7d                	addw	s4,s4,-1
    800001fa:	01bb8463          	beq	s7,s11,80000202 <consoleread+0xb6>
    800001fe:	f80a1fe3          	bnez	s4,8000019c <consoleread+0x50>
    80000202:	0000f517          	auipc	a0,0xf
    80000206:	65e50513          	add	a0,a0,1630 # 8000f860 <cons>
    8000020a:	245000ef          	jal	80000c4e <release>
    8000020e:	414b053b          	subw	a0,s6,s4
    80000212:	a801                	j	80000222 <consoleread+0xd6>
    80000214:	0000f517          	auipc	a0,0xf
    80000218:	64c50513          	add	a0,a0,1612 # 8000f860 <cons>
    8000021c:	233000ef          	jal	80000c4e <release>
    80000220:	557d                	li	a0,-1
    80000222:	70e6                	ld	ra,120(sp)
    80000224:	7446                	ld	s0,112(sp)
    80000226:	74a6                	ld	s1,104(sp)
    80000228:	7906                	ld	s2,96(sp)
    8000022a:	69e6                	ld	s3,88(sp)
    8000022c:	6a46                	ld	s4,80(sp)
    8000022e:	6aa6                	ld	s5,72(sp)
    80000230:	6b06                	ld	s6,64(sp)
    80000232:	7be2                	ld	s7,56(sp)
    80000234:	7c42                	ld	s8,48(sp)
    80000236:	7ca2                	ld	s9,40(sp)
    80000238:	7d02                	ld	s10,32(sp)
    8000023a:	6de2                	ld	s11,24(sp)
    8000023c:	6109                	add	sp,sp,128
    8000023e:	8082                	ret
    80000240:	000a071b          	sext.w	a4,s4
    80000244:	fb677fe3          	bgeu	a4,s6,80000202 <consoleread+0xb6>
    80000248:	0000f717          	auipc	a4,0xf
    8000024c:	6af72823          	sw	a5,1712(a4) # 8000f8f8 <cons+0x98>
    80000250:	bf4d                	j	80000202 <consoleread+0xb6>

0000000080000252 <consputc>:
    80000252:	1141                	add	sp,sp,-16
    80000254:	e406                	sd	ra,8(sp)
    80000256:	e022                	sd	s0,0(sp)
    80000258:	0800                	add	s0,sp,16
    8000025a:	10000793          	li	a5,256
    8000025e:	00f50863          	beq	a0,a5,8000026e <consputc+0x1c>
    80000262:	6aa000ef          	jal	8000090c <uartputc_sync>
    80000266:	60a2                	ld	ra,8(sp)
    80000268:	6402                	ld	s0,0(sp)
    8000026a:	0141                	add	sp,sp,16
    8000026c:	8082                	ret
    8000026e:	4521                	li	a0,8
    80000270:	69c000ef          	jal	8000090c <uartputc_sync>
    80000274:	02000513          	li	a0,32
    80000278:	694000ef          	jal	8000090c <uartputc_sync>
    8000027c:	4521                	li	a0,8
    8000027e:	68e000ef          	jal	8000090c <uartputc_sync>
    80000282:	b7d5                	j	80000266 <consputc+0x14>

0000000080000284 <consoleintr>:
    80000284:	1101                	add	sp,sp,-32
    80000286:	ec06                	sd	ra,24(sp)
    80000288:	e822                	sd	s0,16(sp)
    8000028a:	e426                	sd	s1,8(sp)
    8000028c:	e04a                	sd	s2,0(sp)
    8000028e:	1000                	add	s0,sp,32
    80000290:	84aa                	mv	s1,a0
    80000292:	0000f517          	auipc	a0,0xf
    80000296:	5ce50513          	add	a0,a0,1486 # 8000f860 <cons>
    8000029a:	11d000ef          	jal	80000bb6 <acquire>
    8000029e:	47c1                	li	a5,16
    800002a0:	10f48e63          	beq	s1,a5,800003bc <consoleintr+0x138>
    800002a4:	0297dd63          	bge	a5,s1,800002de <consoleintr+0x5a>
    800002a8:	47d5                	li	a5,21
    800002aa:	0af48463          	beq	s1,a5,80000352 <consoleintr+0xce>
    800002ae:	07f00793          	li	a5,127
    800002b2:	02f49963          	bne	s1,a5,800002e4 <consoleintr+0x60>
    800002b6:	0000f717          	auipc	a4,0xf
    800002ba:	5aa70713          	add	a4,a4,1450 # 8000f860 <cons>
    800002be:	0a072783          	lw	a5,160(a4)
    800002c2:	09c72703          	lw	a4,156(a4)
    800002c6:	0ef70d63          	beq	a4,a5,800003c0 <consoleintr+0x13c>
    800002ca:	37fd                	addw	a5,a5,-1
    800002cc:	0000f717          	auipc	a4,0xf
    800002d0:	62f72a23          	sw	a5,1588(a4) # 8000f900 <cons+0xa0>
    800002d4:	10000513          	li	a0,256
    800002d8:	f7bff0ef          	jal	80000252 <consputc>
    800002dc:	a0d5                	j	800003c0 <consoleintr+0x13c>
    800002de:	47a1                	li	a5,8
    800002e0:	fcf48be3          	beq	s1,a5,800002b6 <consoleintr+0x32>
    800002e4:	ccf1                	beqz	s1,800003c0 <consoleintr+0x13c>
    800002e6:	0000f717          	auipc	a4,0xf
    800002ea:	57a70713          	add	a4,a4,1402 # 8000f860 <cons>
    800002ee:	0a072783          	lw	a5,160(a4)
    800002f2:	09872703          	lw	a4,152(a4)
    800002f6:	9f99                	subw	a5,a5,a4
    800002f8:	07f00713          	li	a4,127
    800002fc:	0cf76263          	bltu	a4,a5,800003c0 <consoleintr+0x13c>
    80000300:	47b5                	li	a5,13
    80000302:	0cf48b63          	beq	s1,a5,800003d8 <consoleintr+0x154>
    80000306:	8526                	mv	a0,s1
    80000308:	f4bff0ef          	jal	80000252 <consputc>
    8000030c:	0000f797          	auipc	a5,0xf
    80000310:	55478793          	add	a5,a5,1364 # 8000f860 <cons>
    80000314:	0a07a683          	lw	a3,160(a5)
    80000318:	0016871b          	addw	a4,a3,1
    8000031c:	0007061b          	sext.w	a2,a4
    80000320:	0ae7a023          	sw	a4,160(a5)
    80000324:	07f6f693          	and	a3,a3,127
    80000328:	97b6                	add	a5,a5,a3
    8000032a:	00978c23          	sb	s1,24(a5)
    8000032e:	47a9                	li	a5,10
    80000330:	0cf48963          	beq	s1,a5,80000402 <consoleintr+0x17e>
    80000334:	4791                	li	a5,4
    80000336:	0cf48663          	beq	s1,a5,80000402 <consoleintr+0x17e>
    8000033a:	0000f797          	auipc	a5,0xf
    8000033e:	52678793          	add	a5,a5,1318 # 8000f860 <cons>
    80000342:	0987a783          	lw	a5,152(a5)
    80000346:	9f1d                	subw	a4,a4,a5
    80000348:	08000793          	li	a5,128
    8000034c:	06f71a63          	bne	a4,a5,800003c0 <consoleintr+0x13c>
    80000350:	a84d                	j	80000402 <consoleintr+0x17e>
    80000352:	0000f717          	auipc	a4,0xf
    80000356:	50e70713          	add	a4,a4,1294 # 8000f860 <cons>
    8000035a:	0a072783          	lw	a5,160(a4)
    8000035e:	09c72703          	lw	a4,156(a4)
    80000362:	04f70f63          	beq	a4,a5,800003c0 <consoleintr+0x13c>
    80000366:	37fd                	addw	a5,a5,-1
    80000368:	0007871b          	sext.w	a4,a5
    8000036c:	07f7f793          	and	a5,a5,127
    80000370:	0000f697          	auipc	a3,0xf
    80000374:	4f068693          	add	a3,a3,1264 # 8000f860 <cons>
    80000378:	97b6                	add	a5,a5,a3
    8000037a:	0187c683          	lbu	a3,24(a5)
    8000037e:	47a9                	li	a5,10
    80000380:	0000f497          	auipc	s1,0xf
    80000384:	4e048493          	add	s1,s1,1248 # 8000f860 <cons>
    80000388:	4929                	li	s2,10
    8000038a:	02f68b63          	beq	a3,a5,800003c0 <consoleintr+0x13c>
    8000038e:	0ae4a023          	sw	a4,160(s1)
    80000392:	10000513          	li	a0,256
    80000396:	ebdff0ef          	jal	80000252 <consputc>
    8000039a:	0a04a783          	lw	a5,160(s1)
    8000039e:	09c4a703          	lw	a4,156(s1)
    800003a2:	00f70f63          	beq	a4,a5,800003c0 <consoleintr+0x13c>
    800003a6:	37fd                	addw	a5,a5,-1
    800003a8:	0007871b          	sext.w	a4,a5
    800003ac:	07f7f793          	and	a5,a5,127
    800003b0:	97a6                	add	a5,a5,s1
    800003b2:	0187c783          	lbu	a5,24(a5)
    800003b6:	fd279ce3          	bne	a5,s2,8000038e <consoleintr+0x10a>
    800003ba:	a019                	j	800003c0 <consoleintr+0x13c>
    800003bc:	6cb010ef          	jal	80002286 <procdump>
    800003c0:	0000f517          	auipc	a0,0xf
    800003c4:	4a050513          	add	a0,a0,1184 # 8000f860 <cons>
    800003c8:	087000ef          	jal	80000c4e <release>
    800003cc:	60e2                	ld	ra,24(sp)
    800003ce:	6442                	ld	s0,16(sp)
    800003d0:	64a2                	ld	s1,8(sp)
    800003d2:	6902                	ld	s2,0(sp)
    800003d4:	6105                	add	sp,sp,32
    800003d6:	8082                	ret
    800003d8:	4529                	li	a0,10
    800003da:	e79ff0ef          	jal	80000252 <consputc>
    800003de:	0000f797          	auipc	a5,0xf
    800003e2:	48278793          	add	a5,a5,1154 # 8000f860 <cons>
    800003e6:	0a07a703          	lw	a4,160(a5)
    800003ea:	0017069b          	addw	a3,a4,1
    800003ee:	0006861b          	sext.w	a2,a3
    800003f2:	0ad7a023          	sw	a3,160(a5)
    800003f6:	07f77713          	and	a4,a4,127
    800003fa:	97ba                	add	a5,a5,a4
    800003fc:	4729                	li	a4,10
    800003fe:	00e78c23          	sb	a4,24(a5)
    80000402:	0000f797          	auipc	a5,0xf
    80000406:	4ec7ad23          	sw	a2,1274(a5) # 8000f8fc <cons+0x9c>
    8000040a:	0000f517          	auipc	a0,0xf
    8000040e:	4ee50513          	add	a0,a0,1262 # 8000f8f8 <cons+0x98>
    80000412:	2cf010ef          	jal	80001ee0 <wakeup>
    80000416:	b76d                	j	800003c0 <consoleintr+0x13c>

0000000080000418 <consoleinit>:
    80000418:	1141                	add	sp,sp,-16
    8000041a:	e406                	sd	ra,8(sp)
    8000041c:	e022                	sd	s0,0(sp)
    8000041e:	0800                	add	s0,sp,16
    80000420:	00007597          	auipc	a1,0x7
    80000424:	bf058593          	add	a1,a1,-1040 # 80007010 <etext+0x10>
    80000428:	0000f517          	auipc	a0,0xf
    8000042c:	43850513          	add	a0,a0,1080 # 8000f860 <cons>
    80000430:	706000ef          	jal	80000b36 <initlock>
    80000434:	3ee000ef          	jal	80000822 <uartinit>
    80000438:	0001f797          	auipc	a5,0x1f
    8000043c:	59878793          	add	a5,a5,1432 # 8001f9d0 <devsw>
    80000440:	00000717          	auipc	a4,0x0
    80000444:	d0c70713          	add	a4,a4,-756 # 8000014c <consoleread>
    80000448:	eb98                	sd	a4,16(a5)
    8000044a:	00000717          	auipc	a4,0x0
    8000044e:	c8670713          	add	a4,a4,-890 # 800000d0 <consolewrite>
    80000452:	ef98                	sd	a4,24(a5)
    80000454:	60a2                	ld	ra,8(sp)
    80000456:	6402                	ld	s0,0(sp)
    80000458:	0141                	add	sp,sp,16
    8000045a:	8082                	ret

000000008000045c <printint>:
    8000045c:	7139                	add	sp,sp,-64
    8000045e:	fc06                	sd	ra,56(sp)
    80000460:	f822                	sd	s0,48(sp)
    80000462:	f426                	sd	s1,40(sp)
    80000464:	f04a                	sd	s2,32(sp)
    80000466:	0080                	add	s0,sp,64
    80000468:	c219                	beqz	a2,8000046e <printint+0x12>
    8000046a:	00054b63          	bltz	a0,80000480 <printint+0x24>
    8000046e:	4881                	li	a7,0
    80000470:	fc840713          	add	a4,s0,-56
    80000474:	4601                	li	a2,0
    80000476:	00007817          	auipc	a6,0x7
    8000047a:	ba280813          	add	a6,a6,-1118 # 80007018 <digits>
    8000047e:	a039                	j	8000048c <printint+0x30>
    80000480:	40a00533          	neg	a0,a0
    80000484:	4885                	li	a7,1
    80000486:	b7ed                	j	80000470 <printint+0x14>
    80000488:	853e                	mv	a0,a5
    8000048a:	8636                	mv	a2,a3
    8000048c:	0016069b          	addw	a3,a2,1
    80000490:	02b577b3          	remu	a5,a0,a1
    80000494:	97c2                	add	a5,a5,a6
    80000496:	0007c783          	lbu	a5,0(a5)
    8000049a:	00f70023          	sb	a5,0(a4)
    8000049e:	02b557b3          	divu	a5,a0,a1
    800004a2:	0705                	add	a4,a4,1
    800004a4:	feb572e3          	bgeu	a0,a1,80000488 <printint+0x2c>
    800004a8:	00088b63          	beqz	a7,800004be <printint+0x62>
    800004ac:	fe040793          	add	a5,s0,-32
    800004b0:	96be                	add	a3,a3,a5
    800004b2:	02d00793          	li	a5,45
    800004b6:	fef68423          	sb	a5,-24(a3)
    800004ba:	0026069b          	addw	a3,a2,2
    800004be:	02d05563          	blez	a3,800004e8 <printint+0x8c>
    800004c2:	fc840793          	add	a5,s0,-56
    800004c6:	00d784b3          	add	s1,a5,a3
    800004ca:	fff78913          	add	s2,a5,-1
    800004ce:	9936                	add	s2,s2,a3
    800004d0:	36fd                	addw	a3,a3,-1
    800004d2:	1682                	sll	a3,a3,0x20
    800004d4:	9281                	srl	a3,a3,0x20
    800004d6:	40d90933          	sub	s2,s2,a3
    800004da:	fff4c503          	lbu	a0,-1(s1)
    800004de:	d75ff0ef          	jal	80000252 <consputc>
    800004e2:	14fd                	add	s1,s1,-1
    800004e4:	ff249be3          	bne	s1,s2,800004da <printint+0x7e>
    800004e8:	70e2                	ld	ra,56(sp)
    800004ea:	7442                	ld	s0,48(sp)
    800004ec:	74a2                	ld	s1,40(sp)
    800004ee:	7902                	ld	s2,32(sp)
    800004f0:	6121                	add	sp,sp,64
    800004f2:	8082                	ret

00000000800004f4 <printf>:
    800004f4:	7131                	add	sp,sp,-192
    800004f6:	fc86                	sd	ra,120(sp)
    800004f8:	f8a2                	sd	s0,112(sp)
    800004fa:	f4a6                	sd	s1,104(sp)
    800004fc:	f0ca                	sd	s2,96(sp)
    800004fe:	ecce                	sd	s3,88(sp)
    80000500:	e8d2                	sd	s4,80(sp)
    80000502:	e4d6                	sd	s5,72(sp)
    80000504:	e0da                	sd	s6,64(sp)
    80000506:	fc5e                	sd	s7,56(sp)
    80000508:	f862                	sd	s8,48(sp)
    8000050a:	f466                	sd	s9,40(sp)
    8000050c:	f06a                	sd	s10,32(sp)
    8000050e:	ec6e                	sd	s11,24(sp)
    80000510:	0100                	add	s0,sp,128
    80000512:	8a2a                	mv	s4,a0
    80000514:	e40c                	sd	a1,8(s0)
    80000516:	e810                	sd	a2,16(s0)
    80000518:	ec14                	sd	a3,24(s0)
    8000051a:	f018                	sd	a4,32(s0)
    8000051c:	f41c                	sd	a5,40(s0)
    8000051e:	03043823          	sd	a6,48(s0)
    80000522:	03143c23          	sd	a7,56(s0)
    80000526:	00007797          	auipc	a5,0x7
    8000052a:	30e78793          	add	a5,a5,782 # 80007834 <panicking>
    8000052e:	439c                	lw	a5,0(a5)
    80000530:	2781                	sext.w	a5,a5
    80000532:	cb9d                	beqz	a5,80000568 <printf+0x74>
    80000534:	00840793          	add	a5,s0,8
    80000538:	f8f43423          	sd	a5,-120(s0)
    8000053c:	000a4503          	lbu	a0,0(s4)
    80000540:	24050363          	beqz	a0,80000786 <printf+0x292>
    80000544:	4981                	li	s3,0
    80000546:	02500a93          	li	s5,37
    8000054a:	06400b13          	li	s6,100
    8000054e:	06c00c13          	li	s8,108
    80000552:	07500c93          	li	s9,117
    80000556:	07800d13          	li	s10,120
    8000055a:	07000d93          	li	s11,112
    8000055e:	00007b97          	auipc	s7,0x7
    80000562:	abab8b93          	add	s7,s7,-1350 # 80007018 <digits>
    80000566:	a01d                	j	8000058c <printf+0x98>
    80000568:	0000f517          	auipc	a0,0xf
    8000056c:	3a050513          	add	a0,a0,928 # 8000f908 <pr>
    80000570:	646000ef          	jal	80000bb6 <acquire>
    80000574:	b7c1                	j	80000534 <printf+0x40>
    80000576:	cddff0ef          	jal	80000252 <consputc>
    8000057a:	84ce                	mv	s1,s3
    8000057c:	0014899b          	addw	s3,s1,1
    80000580:	013a07b3          	add	a5,s4,s3
    80000584:	0007c503          	lbu	a0,0(a5)
    80000588:	1e050f63          	beqz	a0,80000786 <printf+0x292>
    8000058c:	ff5515e3          	bne	a0,s5,80000576 <printf+0x82>
    80000590:	0019849b          	addw	s1,s3,1
    80000594:	009a07b3          	add	a5,s4,s1
    80000598:	0007c903          	lbu	s2,0(a5)
    8000059c:	1e090563          	beqz	s2,80000786 <printf+0x292>
    800005a0:	0017c783          	lbu	a5,1(a5)
    800005a4:	86be                	mv	a3,a5
    800005a6:	c789                	beqz	a5,800005b0 <printf+0xbc>
    800005a8:	009a0733          	add	a4,s4,s1
    800005ac:	00274683          	lbu	a3,2(a4)
    800005b0:	03690863          	beq	s2,s6,800005e0 <printf+0xec>
    800005b4:	05890263          	beq	s2,s8,800005f8 <printf+0x104>
    800005b8:	0d990163          	beq	s2,s9,8000067a <printf+0x186>
    800005bc:	11a90863          	beq	s2,s10,800006cc <printf+0x1d8>
    800005c0:	15b90163          	beq	s2,s11,80000702 <printf+0x20e>
    800005c4:	06300793          	li	a5,99
    800005c8:	16f90963          	beq	s2,a5,8000073a <printf+0x246>
    800005cc:	07300793          	li	a5,115
    800005d0:	16f90f63          	beq	s2,a5,8000074e <printf+0x25a>
    800005d4:	03591c63          	bne	s2,s5,8000060c <printf+0x118>
    800005d8:	8556                	mv	a0,s5
    800005da:	c79ff0ef          	jal	80000252 <consputc>
    800005de:	bf79                	j	8000057c <printf+0x88>
    800005e0:	f8843783          	ld	a5,-120(s0)
    800005e4:	00878713          	add	a4,a5,8
    800005e8:	f8e43423          	sd	a4,-120(s0)
    800005ec:	4605                	li	a2,1
    800005ee:	45a9                	li	a1,10
    800005f0:	4388                	lw	a0,0(a5)
    800005f2:	e6bff0ef          	jal	8000045c <printint>
    800005f6:	b759                	j	8000057c <printf+0x88>
    800005f8:	03678163          	beq	a5,s6,8000061a <printf+0x126>
    800005fc:	03878d63          	beq	a5,s8,80000636 <printf+0x142>
    80000600:	09978a63          	beq	a5,s9,80000694 <printf+0x1a0>
    80000604:	03878b63          	beq	a5,s8,8000063a <printf+0x146>
    80000608:	0da78f63          	beq	a5,s10,800006e6 <printf+0x1f2>
    8000060c:	8556                	mv	a0,s5
    8000060e:	c45ff0ef          	jal	80000252 <consputc>
    80000612:	854a                	mv	a0,s2
    80000614:	c3fff0ef          	jal	80000252 <consputc>
    80000618:	b795                	j	8000057c <printf+0x88>
    8000061a:	f8843783          	ld	a5,-120(s0)
    8000061e:	00878713          	add	a4,a5,8
    80000622:	f8e43423          	sd	a4,-120(s0)
    80000626:	4605                	li	a2,1
    80000628:	45a9                	li	a1,10
    8000062a:	6388                	ld	a0,0(a5)
    8000062c:	e31ff0ef          	jal	8000045c <printint>
    80000630:	0029849b          	addw	s1,s3,2
    80000634:	b7a1                	j	8000057c <printf+0x88>
    80000636:	03668463          	beq	a3,s6,8000065e <printf+0x16a>
    8000063a:	07968b63          	beq	a3,s9,800006b0 <printf+0x1bc>
    8000063e:	fda697e3          	bne	a3,s10,8000060c <printf+0x118>
    80000642:	f8843783          	ld	a5,-120(s0)
    80000646:	00878713          	add	a4,a5,8
    8000064a:	f8e43423          	sd	a4,-120(s0)
    8000064e:	4601                	li	a2,0
    80000650:	45c1                	li	a1,16
    80000652:	6388                	ld	a0,0(a5)
    80000654:	e09ff0ef          	jal	8000045c <printint>
    80000658:	0039849b          	addw	s1,s3,3
    8000065c:	b705                	j	8000057c <printf+0x88>
    8000065e:	f8843783          	ld	a5,-120(s0)
    80000662:	00878713          	add	a4,a5,8
    80000666:	f8e43423          	sd	a4,-120(s0)
    8000066a:	4605                	li	a2,1
    8000066c:	45a9                	li	a1,10
    8000066e:	6388                	ld	a0,0(a5)
    80000670:	dedff0ef          	jal	8000045c <printint>
    80000674:	0039849b          	addw	s1,s3,3
    80000678:	b711                	j	8000057c <printf+0x88>
    8000067a:	f8843783          	ld	a5,-120(s0)
    8000067e:	00878713          	add	a4,a5,8
    80000682:	f8e43423          	sd	a4,-120(s0)
    80000686:	4601                	li	a2,0
    80000688:	45a9                	li	a1,10
    8000068a:	0007e503          	lwu	a0,0(a5)
    8000068e:	dcfff0ef          	jal	8000045c <printint>
    80000692:	b5ed                	j	8000057c <printf+0x88>
    80000694:	f8843783          	ld	a5,-120(s0)
    80000698:	00878713          	add	a4,a5,8
    8000069c:	f8e43423          	sd	a4,-120(s0)
    800006a0:	4601                	li	a2,0
    800006a2:	45a9                	li	a1,10
    800006a4:	6388                	ld	a0,0(a5)
    800006a6:	db7ff0ef          	jal	8000045c <printint>
    800006aa:	0029849b          	addw	s1,s3,2
    800006ae:	b5f9                	j	8000057c <printf+0x88>
    800006b0:	f8843783          	ld	a5,-120(s0)
    800006b4:	00878713          	add	a4,a5,8
    800006b8:	f8e43423          	sd	a4,-120(s0)
    800006bc:	4601                	li	a2,0
    800006be:	45a9                	li	a1,10
    800006c0:	6388                	ld	a0,0(a5)
    800006c2:	d9bff0ef          	jal	8000045c <printint>
    800006c6:	0039849b          	addw	s1,s3,3
    800006ca:	bd4d                	j	8000057c <printf+0x88>
    800006cc:	f8843783          	ld	a5,-120(s0)
    800006d0:	00878713          	add	a4,a5,8
    800006d4:	f8e43423          	sd	a4,-120(s0)
    800006d8:	4601                	li	a2,0
    800006da:	45c1                	li	a1,16
    800006dc:	0007e503          	lwu	a0,0(a5)
    800006e0:	d7dff0ef          	jal	8000045c <printint>
    800006e4:	bd61                	j	8000057c <printf+0x88>
    800006e6:	f8843783          	ld	a5,-120(s0)
    800006ea:	00878713          	add	a4,a5,8
    800006ee:	f8e43423          	sd	a4,-120(s0)
    800006f2:	4601                	li	a2,0
    800006f4:	45c1                	li	a1,16
    800006f6:	6388                	ld	a0,0(a5)
    800006f8:	d65ff0ef          	jal	8000045c <printint>
    800006fc:	0029849b          	addw	s1,s3,2
    80000700:	bdb5                	j	8000057c <printf+0x88>
    80000702:	f8843783          	ld	a5,-120(s0)
    80000706:	00878713          	add	a4,a5,8
    8000070a:	f8e43423          	sd	a4,-120(s0)
    8000070e:	0007b983          	ld	s3,0(a5)
    80000712:	03000513          	li	a0,48
    80000716:	b3dff0ef          	jal	80000252 <consputc>
    8000071a:	856a                	mv	a0,s10
    8000071c:	b37ff0ef          	jal	80000252 <consputc>
    80000720:	4941                	li	s2,16
    80000722:	03c9d793          	srl	a5,s3,0x3c
    80000726:	97de                	add	a5,a5,s7
    80000728:	0007c503          	lbu	a0,0(a5)
    8000072c:	b27ff0ef          	jal	80000252 <consputc>
    80000730:	0992                	sll	s3,s3,0x4
    80000732:	397d                	addw	s2,s2,-1
    80000734:	fe0917e3          	bnez	s2,80000722 <printf+0x22e>
    80000738:	b591                	j	8000057c <printf+0x88>
    8000073a:	f8843783          	ld	a5,-120(s0)
    8000073e:	00878713          	add	a4,a5,8
    80000742:	f8e43423          	sd	a4,-120(s0)
    80000746:	4388                	lw	a0,0(a5)
    80000748:	b0bff0ef          	jal	80000252 <consputc>
    8000074c:	bd05                	j	8000057c <printf+0x88>
    8000074e:	f8843783          	ld	a5,-120(s0)
    80000752:	00878713          	add	a4,a5,8
    80000756:	f8e43423          	sd	a4,-120(s0)
    8000075a:	0007b903          	ld	s2,0(a5)
    8000075e:	00090d63          	beqz	s2,80000778 <printf+0x284>
    80000762:	00094503          	lbu	a0,0(s2)
    80000766:	e0050be3          	beqz	a0,8000057c <printf+0x88>
    8000076a:	ae9ff0ef          	jal	80000252 <consputc>
    8000076e:	0905                	add	s2,s2,1
    80000770:	00094503          	lbu	a0,0(s2)
    80000774:	f97d                	bnez	a0,8000076a <printf+0x276>
    80000776:	b519                	j	8000057c <printf+0x88>
    80000778:	00007917          	auipc	s2,0x7
    8000077c:	8b890913          	add	s2,s2,-1864 # 80007030 <digits+0x18>
    80000780:	02800513          	li	a0,40
    80000784:	b7dd                	j	8000076a <printf+0x276>
    80000786:	00007797          	auipc	a5,0x7
    8000078a:	0ae78793          	add	a5,a5,174 # 80007834 <panicking>
    8000078e:	439c                	lw	a5,0(a5)
    80000790:	2781                	sext.w	a5,a5
    80000792:	c38d                	beqz	a5,800007b4 <printf+0x2c0>
    80000794:	4501                	li	a0,0
    80000796:	70e6                	ld	ra,120(sp)
    80000798:	7446                	ld	s0,112(sp)
    8000079a:	74a6                	ld	s1,104(sp)
    8000079c:	7906                	ld	s2,96(sp)
    8000079e:	69e6                	ld	s3,88(sp)
    800007a0:	6a46                	ld	s4,80(sp)
    800007a2:	6aa6                	ld	s5,72(sp)
    800007a4:	6b06                	ld	s6,64(sp)
    800007a6:	7be2                	ld	s7,56(sp)
    800007a8:	7c42                	ld	s8,48(sp)
    800007aa:	7ca2                	ld	s9,40(sp)
    800007ac:	7d02                	ld	s10,32(sp)
    800007ae:	6de2                	ld	s11,24(sp)
    800007b0:	6129                	add	sp,sp,192
    800007b2:	8082                	ret
    800007b4:	0000f517          	auipc	a0,0xf
    800007b8:	15450513          	add	a0,a0,340 # 8000f908 <pr>
    800007bc:	492000ef          	jal	80000c4e <release>
    800007c0:	bfd1                	j	80000794 <printf+0x2a0>

00000000800007c2 <panic>:
    800007c2:	1101                	add	sp,sp,-32
    800007c4:	ec06                	sd	ra,24(sp)
    800007c6:	e822                	sd	s0,16(sp)
    800007c8:	e426                	sd	s1,8(sp)
    800007ca:	e04a                	sd	s2,0(sp)
    800007cc:	1000                	add	s0,sp,32
    800007ce:	892a                	mv	s2,a0
    800007d0:	4485                	li	s1,1
    800007d2:	00007797          	auipc	a5,0x7
    800007d6:	0697a123          	sw	s1,98(a5) # 80007834 <panicking>
    800007da:	00007517          	auipc	a0,0x7
    800007de:	85e50513          	add	a0,a0,-1954 # 80007038 <digits+0x20>
    800007e2:	d13ff0ef          	jal	800004f4 <printf>
    800007e6:	85ca                	mv	a1,s2
    800007e8:	00007517          	auipc	a0,0x7
    800007ec:	85850513          	add	a0,a0,-1960 # 80007040 <digits+0x28>
    800007f0:	d05ff0ef          	jal	800004f4 <printf>
    800007f4:	00007797          	auipc	a5,0x7
    800007f8:	0297ae23          	sw	s1,60(a5) # 80007830 <panicked>
    800007fc:	a001                	j	800007fc <panic+0x3a>

00000000800007fe <printfinit>:
    800007fe:	1141                	add	sp,sp,-16
    80000800:	e406                	sd	ra,8(sp)
    80000802:	e022                	sd	s0,0(sp)
    80000804:	0800                	add	s0,sp,16
    80000806:	00007597          	auipc	a1,0x7
    8000080a:	84258593          	add	a1,a1,-1982 # 80007048 <digits+0x30>
    8000080e:	0000f517          	auipc	a0,0xf
    80000812:	0fa50513          	add	a0,a0,250 # 8000f908 <pr>
    80000816:	320000ef          	jal	80000b36 <initlock>
    8000081a:	60a2                	ld	ra,8(sp)
    8000081c:	6402                	ld	s0,0(sp)
    8000081e:	0141                	add	sp,sp,16
    80000820:	8082                	ret

0000000080000822 <uartinit>:
    80000822:	1141                	add	sp,sp,-16
    80000824:	e406                	sd	ra,8(sp)
    80000826:	e022                	sd	s0,0(sp)
    80000828:	0800                	add	s0,sp,16
    8000082a:	100007b7          	lui	a5,0x10000
    8000082e:	000780a3          	sb	zero,1(a5) # 10000001 <_entry-0x6fffffff>
    80000832:	f8000713          	li	a4,-128
    80000836:	00e781a3          	sb	a4,3(a5)
    8000083a:	470d                	li	a4,3
    8000083c:	00e78023          	sb	a4,0(a5)
    80000840:	000780a3          	sb	zero,1(a5)
    80000844:	00e781a3          	sb	a4,3(a5)
    80000848:	469d                	li	a3,7
    8000084a:	00d78123          	sb	a3,2(a5)
    8000084e:	00e780a3          	sb	a4,1(a5)
    80000852:	00006597          	auipc	a1,0x6
    80000856:	7fe58593          	add	a1,a1,2046 # 80007050 <digits+0x38>
    8000085a:	0000f517          	auipc	a0,0xf
    8000085e:	0c650513          	add	a0,a0,198 # 8000f920 <tx_lock>
    80000862:	2d4000ef          	jal	80000b36 <initlock>
    80000866:	60a2                	ld	ra,8(sp)
    80000868:	6402                	ld	s0,0(sp)
    8000086a:	0141                	add	sp,sp,16
    8000086c:	8082                	ret

000000008000086e <uartwrite>:
    8000086e:	715d                	add	sp,sp,-80
    80000870:	e486                	sd	ra,72(sp)
    80000872:	e0a2                	sd	s0,64(sp)
    80000874:	fc26                	sd	s1,56(sp)
    80000876:	f84a                	sd	s2,48(sp)
    80000878:	f44e                	sd	s3,40(sp)
    8000087a:	f052                	sd	s4,32(sp)
    8000087c:	ec56                	sd	s5,24(sp)
    8000087e:	e85a                	sd	s6,16(sp)
    80000880:	e45e                	sd	s7,8(sp)
    80000882:	0880                	add	s0,sp,80
    80000884:	84aa                	mv	s1,a0
    80000886:	8aae                	mv	s5,a1
    80000888:	0000f517          	auipc	a0,0xf
    8000088c:	09850513          	add	a0,a0,152 # 8000f920 <tx_lock>
    80000890:	326000ef          	jal	80000bb6 <acquire>
    80000894:	05505b63          	blez	s5,800008ea <uartwrite+0x7c>
    80000898:	8a26                	mv	s4,s1
    8000089a:	0485                	add	s1,s1,1
    8000089c:	3afd                	addw	s5,s5,-1
    8000089e:	1a82                	sll	s5,s5,0x20
    800008a0:	020ada93          	srl	s5,s5,0x20
    800008a4:	9aa6                	add	s5,s5,s1
    800008a6:	00007497          	auipc	s1,0x7
    800008aa:	f9648493          	add	s1,s1,-106 # 8000783c <tx_busy>
    800008ae:	0000f997          	auipc	s3,0xf
    800008b2:	07298993          	add	s3,s3,114 # 8000f920 <tx_lock>
    800008b6:	00007917          	auipc	s2,0x7
    800008ba:	f8290913          	add	s2,s2,-126 # 80007838 <tx_chan>
    800008be:	10000bb7          	lui	s7,0x10000
    800008c2:	4b05                	li	s6,1
    800008c4:	a811                	j	800008d8 <uartwrite+0x6a>
    800008c6:	000a4783          	lbu	a5,0(s4)
    800008ca:	00fb8023          	sb	a5,0(s7) # 10000000 <_entry-0x70000000>
    800008ce:	0164a023          	sw	s6,0(s1)
    800008d2:	0a05                	add	s4,s4,1
    800008d4:	015a0b63          	beq	s4,s5,800008ea <uartwrite+0x7c>
    800008d8:	409c                	lw	a5,0(s1)
    800008da:	d7f5                	beqz	a5,800008c6 <uartwrite+0x58>
    800008dc:	85ce                	mv	a1,s3
    800008de:	854a                	mv	a0,s2
    800008e0:	5b4010ef          	jal	80001e94 <sleep>
    800008e4:	409c                	lw	a5,0(s1)
    800008e6:	fbfd                	bnez	a5,800008dc <uartwrite+0x6e>
    800008e8:	bff9                	j	800008c6 <uartwrite+0x58>
    800008ea:	0000f517          	auipc	a0,0xf
    800008ee:	03650513          	add	a0,a0,54 # 8000f920 <tx_lock>
    800008f2:	35c000ef          	jal	80000c4e <release>
    800008f6:	60a6                	ld	ra,72(sp)
    800008f8:	6406                	ld	s0,64(sp)
    800008fa:	74e2                	ld	s1,56(sp)
    800008fc:	7942                	ld	s2,48(sp)
    800008fe:	79a2                	ld	s3,40(sp)
    80000900:	7a02                	ld	s4,32(sp)
    80000902:	6ae2                	ld	s5,24(sp)
    80000904:	6b42                	ld	s6,16(sp)
    80000906:	6ba2                	ld	s7,8(sp)
    80000908:	6161                	add	sp,sp,80
    8000090a:	8082                	ret

000000008000090c <uartputc_sync>:
    8000090c:	1101                	add	sp,sp,-32
    8000090e:	ec06                	sd	ra,24(sp)
    80000910:	e822                	sd	s0,16(sp)
    80000912:	e426                	sd	s1,8(sp)
    80000914:	1000                	add	s0,sp,32
    80000916:	84aa                	mv	s1,a0
    80000918:	00007797          	auipc	a5,0x7
    8000091c:	f1c78793          	add	a5,a5,-228 # 80007834 <panicking>
    80000920:	439c                	lw	a5,0(a5)
    80000922:	2781                	sext.w	a5,a5
    80000924:	cb99                	beqz	a5,8000093a <uartputc_sync+0x2e>
    80000926:	00007797          	auipc	a5,0x7
    8000092a:	f0a78793          	add	a5,a5,-246 # 80007830 <panicked>
    8000092e:	439c                	lw	a5,0(a5)
    80000930:	2781                	sext.w	a5,a5
    80000932:	10000737          	lui	a4,0x10000
    80000936:	c789                	beqz	a5,80000940 <uartputc_sync+0x34>
    80000938:	a001                	j	80000938 <uartputc_sync+0x2c>
    8000093a:	23c000ef          	jal	80000b76 <push_off>
    8000093e:	b7e5                	j	80000926 <uartputc_sync+0x1a>
    80000940:	00574783          	lbu	a5,5(a4) # 10000005 <_entry-0x6ffffffb>
    80000944:	0ff7f793          	zext.b	a5,a5
    80000948:	0207f793          	and	a5,a5,32
    8000094c:	dbf5                	beqz	a5,80000940 <uartputc_sync+0x34>
    8000094e:	0ff4f793          	zext.b	a5,s1
    80000952:	10000737          	lui	a4,0x10000
    80000956:	00f70023          	sb	a5,0(a4) # 10000000 <_entry-0x70000000>
    8000095a:	00007797          	auipc	a5,0x7
    8000095e:	eda78793          	add	a5,a5,-294 # 80007834 <panicking>
    80000962:	439c                	lw	a5,0(a5)
    80000964:	2781                	sext.w	a5,a5
    80000966:	c791                	beqz	a5,80000972 <uartputc_sync+0x66>
    80000968:	60e2                	ld	ra,24(sp)
    8000096a:	6442                	ld	s0,16(sp)
    8000096c:	64a2                	ld	s1,8(sp)
    8000096e:	6105                	add	sp,sp,32
    80000970:	8082                	ret
    80000972:	288000ef          	jal	80000bfa <pop_off>
    80000976:	bfcd                	j	80000968 <uartputc_sync+0x5c>

0000000080000978 <uartgetc>:
    80000978:	1141                	add	sp,sp,-16
    8000097a:	e422                	sd	s0,8(sp)
    8000097c:	0800                	add	s0,sp,16
    8000097e:	100007b7          	lui	a5,0x10000
    80000982:	0057c783          	lbu	a5,5(a5) # 10000005 <_entry-0x6ffffffb>
    80000986:	8b85                	and	a5,a5,1
    80000988:	cb81                	beqz	a5,80000998 <uartgetc+0x20>
    8000098a:	100007b7          	lui	a5,0x10000
    8000098e:	0007c503          	lbu	a0,0(a5) # 10000000 <_entry-0x70000000>
    80000992:	6422                	ld	s0,8(sp)
    80000994:	0141                	add	sp,sp,16
    80000996:	8082                	ret
    80000998:	557d                	li	a0,-1
    8000099a:	bfe5                	j	80000992 <uartgetc+0x1a>

000000008000099c <uartintr>:
    8000099c:	1101                	add	sp,sp,-32
    8000099e:	ec06                	sd	ra,24(sp)
    800009a0:	e822                	sd	s0,16(sp)
    800009a2:	e426                	sd	s1,8(sp)
    800009a4:	1000                	add	s0,sp,32
    800009a6:	100004b7          	lui	s1,0x10000
    800009aa:	0024c783          	lbu	a5,2(s1) # 10000002 <_entry-0x6ffffffe>
    800009ae:	0000f517          	auipc	a0,0xf
    800009b2:	f7250513          	add	a0,a0,-142 # 8000f920 <tx_lock>
    800009b6:	200000ef          	jal	80000bb6 <acquire>
    800009ba:	0054c783          	lbu	a5,5(s1)
    800009be:	0ff7f793          	zext.b	a5,a5
    800009c2:	0207f793          	and	a5,a5,32
    800009c6:	ef99                	bnez	a5,800009e4 <uartintr+0x48>
    800009c8:	0000f517          	auipc	a0,0xf
    800009cc:	f5850513          	add	a0,a0,-168 # 8000f920 <tx_lock>
    800009d0:	27e000ef          	jal	80000c4e <release>
    800009d4:	54fd                	li	s1,-1
    800009d6:	fa3ff0ef          	jal	80000978 <uartgetc>
    800009da:	02950063          	beq	a0,s1,800009fa <uartintr+0x5e>
    800009de:	8a7ff0ef          	jal	80000284 <consoleintr>
    800009e2:	bfd5                	j	800009d6 <uartintr+0x3a>
    800009e4:	00007797          	auipc	a5,0x7
    800009e8:	e407ac23          	sw	zero,-424(a5) # 8000783c <tx_busy>
    800009ec:	00007517          	auipc	a0,0x7
    800009f0:	e4c50513          	add	a0,a0,-436 # 80007838 <tx_chan>
    800009f4:	4ec010ef          	jal	80001ee0 <wakeup>
    800009f8:	bfc1                	j	800009c8 <uartintr+0x2c>
    800009fa:	60e2                	ld	ra,24(sp)
    800009fc:	6442                	ld	s0,16(sp)
    800009fe:	64a2                	ld	s1,8(sp)
    80000a00:	6105                	add	sp,sp,32
    80000a02:	8082                	ret

0000000080000a04 <kfree>:
    80000a04:	1101                	add	sp,sp,-32
    80000a06:	ec06                	sd	ra,24(sp)
    80000a08:	e822                	sd	s0,16(sp)
    80000a0a:	e426                	sd	s1,8(sp)
    80000a0c:	e04a                	sd	s2,0(sp)
    80000a0e:	1000                	add	s0,sp,32
    80000a10:	03451793          	sll	a5,a0,0x34
    80000a14:	e7a9                	bnez	a5,80000a5e <kfree+0x5a>
    80000a16:	84aa                	mv	s1,a0
    80000a18:	00020797          	auipc	a5,0x20
    80000a1c:	15078793          	add	a5,a5,336 # 80020b68 <end>
    80000a20:	02f56f63          	bltu	a0,a5,80000a5e <kfree+0x5a>
    80000a24:	47c5                	li	a5,17
    80000a26:	07ee                	sll	a5,a5,0x1b
    80000a28:	02f57b63          	bgeu	a0,a5,80000a5e <kfree+0x5a>
    80000a2c:	6605                	lui	a2,0x1
    80000a2e:	4585                	li	a1,1
    80000a30:	25a000ef          	jal	80000c8a <memset>
    80000a34:	0000f917          	auipc	s2,0xf
    80000a38:	f0490913          	add	s2,s2,-252 # 8000f938 <kmem>
    80000a3c:	854a                	mv	a0,s2
    80000a3e:	178000ef          	jal	80000bb6 <acquire>
    80000a42:	01893783          	ld	a5,24(s2)
    80000a46:	e09c                	sd	a5,0(s1)
    80000a48:	00993c23          	sd	s1,24(s2)
    80000a4c:	854a                	mv	a0,s2
    80000a4e:	200000ef          	jal	80000c4e <release>
    80000a52:	60e2                	ld	ra,24(sp)
    80000a54:	6442                	ld	s0,16(sp)
    80000a56:	64a2                	ld	s1,8(sp)
    80000a58:	6902                	ld	s2,0(sp)
    80000a5a:	6105                	add	sp,sp,32
    80000a5c:	8082                	ret
    80000a5e:	00006517          	auipc	a0,0x6
    80000a62:	5fa50513          	add	a0,a0,1530 # 80007058 <digits+0x40>
    80000a66:	d5dff0ef          	jal	800007c2 <panic>

0000000080000a6a <freerange>:
    80000a6a:	7179                	add	sp,sp,-48
    80000a6c:	f406                	sd	ra,40(sp)
    80000a6e:	f022                	sd	s0,32(sp)
    80000a70:	ec26                	sd	s1,24(sp)
    80000a72:	e84a                	sd	s2,16(sp)
    80000a74:	e44e                	sd	s3,8(sp)
    80000a76:	e052                	sd	s4,0(sp)
    80000a78:	1800                	add	s0,sp,48
    80000a7a:	6705                	lui	a4,0x1
    80000a7c:	fff70793          	add	a5,a4,-1 # fff <_entry-0x7ffff001>
    80000a80:	00f504b3          	add	s1,a0,a5
    80000a84:	77fd                	lui	a5,0xfffff
    80000a86:	8cfd                	and	s1,s1,a5
    80000a88:	94ba                	add	s1,s1,a4
    80000a8a:	0095ec63          	bltu	a1,s1,80000aa2 <freerange+0x38>
    80000a8e:	892e                	mv	s2,a1
    80000a90:	7a7d                	lui	s4,0xfffff
    80000a92:	6985                	lui	s3,0x1
    80000a94:	01448533          	add	a0,s1,s4
    80000a98:	f6dff0ef          	jal	80000a04 <kfree>
    80000a9c:	94ce                	add	s1,s1,s3
    80000a9e:	fe997be3          	bgeu	s2,s1,80000a94 <freerange+0x2a>
    80000aa2:	70a2                	ld	ra,40(sp)
    80000aa4:	7402                	ld	s0,32(sp)
    80000aa6:	64e2                	ld	s1,24(sp)
    80000aa8:	6942                	ld	s2,16(sp)
    80000aaa:	69a2                	ld	s3,8(sp)
    80000aac:	6a02                	ld	s4,0(sp)
    80000aae:	6145                	add	sp,sp,48
    80000ab0:	8082                	ret

0000000080000ab2 <kinit>:
    80000ab2:	1141                	add	sp,sp,-16
    80000ab4:	e406                	sd	ra,8(sp)
    80000ab6:	e022                	sd	s0,0(sp)
    80000ab8:	0800                	add	s0,sp,16
    80000aba:	00006597          	auipc	a1,0x6
    80000abe:	5a658593          	add	a1,a1,1446 # 80007060 <digits+0x48>
    80000ac2:	0000f517          	auipc	a0,0xf
    80000ac6:	e7650513          	add	a0,a0,-394 # 8000f938 <kmem>
    80000aca:	06c000ef          	jal	80000b36 <initlock>
    80000ace:	45c5                	li	a1,17
    80000ad0:	05ee                	sll	a1,a1,0x1b
    80000ad2:	00020517          	auipc	a0,0x20
    80000ad6:	09650513          	add	a0,a0,150 # 80020b68 <end>
    80000ada:	f91ff0ef          	jal	80000a6a <freerange>
    80000ade:	60a2                	ld	ra,8(sp)
    80000ae0:	6402                	ld	s0,0(sp)
    80000ae2:	0141                	add	sp,sp,16
    80000ae4:	8082                	ret

0000000080000ae6 <kalloc>:
    80000ae6:	1101                	add	sp,sp,-32
    80000ae8:	ec06                	sd	ra,24(sp)
    80000aea:	e822                	sd	s0,16(sp)
    80000aec:	e426                	sd	s1,8(sp)
    80000aee:	1000                	add	s0,sp,32
    80000af0:	0000f497          	auipc	s1,0xf
    80000af4:	e4848493          	add	s1,s1,-440 # 8000f938 <kmem>
    80000af8:	8526                	mv	a0,s1
    80000afa:	0bc000ef          	jal	80000bb6 <acquire>
    80000afe:	6c84                	ld	s1,24(s1)
    80000b00:	c485                	beqz	s1,80000b28 <kalloc+0x42>
    80000b02:	609c                	ld	a5,0(s1)
    80000b04:	0000f517          	auipc	a0,0xf
    80000b08:	e3450513          	add	a0,a0,-460 # 8000f938 <kmem>
    80000b0c:	ed1c                	sd	a5,24(a0)
    80000b0e:	140000ef          	jal	80000c4e <release>
    80000b12:	6605                	lui	a2,0x1
    80000b14:	4595                	li	a1,5
    80000b16:	8526                	mv	a0,s1
    80000b18:	172000ef          	jal	80000c8a <memset>
    80000b1c:	8526                	mv	a0,s1
    80000b1e:	60e2                	ld	ra,24(sp)
    80000b20:	6442                	ld	s0,16(sp)
    80000b22:	64a2                	ld	s1,8(sp)
    80000b24:	6105                	add	sp,sp,32
    80000b26:	8082                	ret
    80000b28:	0000f517          	auipc	a0,0xf
    80000b2c:	e1050513          	add	a0,a0,-496 # 8000f938 <kmem>
    80000b30:	11e000ef          	jal	80000c4e <release>
    80000b34:	b7e5                	j	80000b1c <kalloc+0x36>

0000000080000b36 <initlock>:
    80000b36:	1141                	add	sp,sp,-16
    80000b38:	e422                	sd	s0,8(sp)
    80000b3a:	0800                	add	s0,sp,16
    80000b3c:	e50c                	sd	a1,8(a0)
    80000b3e:	00052023          	sw	zero,0(a0)
    80000b42:	00053823          	sd	zero,16(a0)
    80000b46:	6422                	ld	s0,8(sp)
    80000b48:	0141                	add	sp,sp,16
    80000b4a:	8082                	ret

0000000080000b4c <holding>:
    80000b4c:	411c                	lw	a5,0(a0)
    80000b4e:	e399                	bnez	a5,80000b54 <holding+0x8>
    80000b50:	4501                	li	a0,0
    80000b52:	8082                	ret
    80000b54:	1101                	add	sp,sp,-32
    80000b56:	ec06                	sd	ra,24(sp)
    80000b58:	e822                	sd	s0,16(sp)
    80000b5a:	e426                	sd	s1,8(sp)
    80000b5c:	1000                	add	s0,sp,32
    80000b5e:	6904                	ld	s1,16(a0)
    80000b60:	511000ef          	jal	80001870 <mycpu>
    80000b64:	40a48533          	sub	a0,s1,a0
    80000b68:	00153513          	seqz	a0,a0
    80000b6c:	60e2                	ld	ra,24(sp)
    80000b6e:	6442                	ld	s0,16(sp)
    80000b70:	64a2                	ld	s1,8(sp)
    80000b72:	6105                	add	sp,sp,32
    80000b74:	8082                	ret

0000000080000b76 <push_off>:
    80000b76:	1101                	add	sp,sp,-32
    80000b78:	ec06                	sd	ra,24(sp)
    80000b7a:	e822                	sd	s0,16(sp)
    80000b7c:	e426                	sd	s1,8(sp)
    80000b7e:	1000                	add	s0,sp,32
    80000b80:	100024f3          	csrr	s1,sstatus
    80000b84:	100027f3          	csrr	a5,sstatus
    80000b88:	9bf5                	and	a5,a5,-3
    80000b8a:	10079073          	csrw	sstatus,a5
    80000b8e:	4e3000ef          	jal	80001870 <mycpu>
    80000b92:	5d3c                	lw	a5,120(a0)
    80000b94:	cb99                	beqz	a5,80000baa <push_off+0x34>
    80000b96:	4db000ef          	jal	80001870 <mycpu>
    80000b9a:	5d3c                	lw	a5,120(a0)
    80000b9c:	2785                	addw	a5,a5,1 # fffffffffffff001 <end+0xffffffff7ffde499>
    80000b9e:	dd3c                	sw	a5,120(a0)
    80000ba0:	60e2                	ld	ra,24(sp)
    80000ba2:	6442                	ld	s0,16(sp)
    80000ba4:	64a2                	ld	s1,8(sp)
    80000ba6:	6105                	add	sp,sp,32
    80000ba8:	8082                	ret
    80000baa:	4c7000ef          	jal	80001870 <mycpu>
    80000bae:	8085                	srl	s1,s1,0x1
    80000bb0:	8885                	and	s1,s1,1
    80000bb2:	dd64                	sw	s1,124(a0)
    80000bb4:	b7cd                	j	80000b96 <push_off+0x20>

0000000080000bb6 <acquire>:
    80000bb6:	1101                	add	sp,sp,-32
    80000bb8:	ec06                	sd	ra,24(sp)
    80000bba:	e822                	sd	s0,16(sp)
    80000bbc:	e426                	sd	s1,8(sp)
    80000bbe:	1000                	add	s0,sp,32
    80000bc0:	84aa                	mv	s1,a0
    80000bc2:	fb5ff0ef          	jal	80000b76 <push_off>
    80000bc6:	8526                	mv	a0,s1
    80000bc8:	f85ff0ef          	jal	80000b4c <holding>
    80000bcc:	4705                	li	a4,1
    80000bce:	e105                	bnez	a0,80000bee <acquire+0x38>
    80000bd0:	87ba                	mv	a5,a4
    80000bd2:	0cf4a7af          	amoswap.w.aq	a5,a5,(s1)
    80000bd6:	2781                	sext.w	a5,a5
    80000bd8:	ffe5                	bnez	a5,80000bd0 <acquire+0x1a>
    80000bda:	0ff0000f          	fence
    80000bde:	493000ef          	jal	80001870 <mycpu>
    80000be2:	e888                	sd	a0,16(s1)
    80000be4:	60e2                	ld	ra,24(sp)
    80000be6:	6442                	ld	s0,16(sp)
    80000be8:	64a2                	ld	s1,8(sp)
    80000bea:	6105                	add	sp,sp,32
    80000bec:	8082                	ret
    80000bee:	00006517          	auipc	a0,0x6
    80000bf2:	47a50513          	add	a0,a0,1146 # 80007068 <digits+0x50>
    80000bf6:	bcdff0ef          	jal	800007c2 <panic>

0000000080000bfa <pop_off>:
    80000bfa:	1141                	add	sp,sp,-16
    80000bfc:	e406                	sd	ra,8(sp)
    80000bfe:	e022                	sd	s0,0(sp)
    80000c00:	0800                	add	s0,sp,16
    80000c02:	46f000ef          	jal	80001870 <mycpu>
    80000c06:	100027f3          	csrr	a5,sstatus
    80000c0a:	8b89                	and	a5,a5,2
    80000c0c:	e78d                	bnez	a5,80000c36 <pop_off+0x3c>
    80000c0e:	5d3c                	lw	a5,120(a0)
    80000c10:	02f05963          	blez	a5,80000c42 <pop_off+0x48>
    80000c14:	37fd                	addw	a5,a5,-1
    80000c16:	0007871b          	sext.w	a4,a5
    80000c1a:	dd3c                	sw	a5,120(a0)
    80000c1c:	eb09                	bnez	a4,80000c2e <pop_off+0x34>
    80000c1e:	5d7c                	lw	a5,124(a0)
    80000c20:	c799                	beqz	a5,80000c2e <pop_off+0x34>
    80000c22:	100027f3          	csrr	a5,sstatus
    80000c26:	0027e793          	or	a5,a5,2
    80000c2a:	10079073          	csrw	sstatus,a5
    80000c2e:	60a2                	ld	ra,8(sp)
    80000c30:	6402                	ld	s0,0(sp)
    80000c32:	0141                	add	sp,sp,16
    80000c34:	8082                	ret
    80000c36:	00006517          	auipc	a0,0x6
    80000c3a:	43a50513          	add	a0,a0,1082 # 80007070 <digits+0x58>
    80000c3e:	b85ff0ef          	jal	800007c2 <panic>
    80000c42:	00006517          	auipc	a0,0x6
    80000c46:	44650513          	add	a0,a0,1094 # 80007088 <digits+0x70>
    80000c4a:	b79ff0ef          	jal	800007c2 <panic>

0000000080000c4e <release>:
    80000c4e:	1101                	add	sp,sp,-32
    80000c50:	ec06                	sd	ra,24(sp)
    80000c52:	e822                	sd	s0,16(sp)
    80000c54:	e426                	sd	s1,8(sp)
    80000c56:	1000                	add	s0,sp,32
    80000c58:	84aa                	mv	s1,a0
    80000c5a:	ef3ff0ef          	jal	80000b4c <holding>
    80000c5e:	c105                	beqz	a0,80000c7e <release+0x30>
    80000c60:	0004b823          	sd	zero,16(s1)
    80000c64:	0ff0000f          	fence
    80000c68:	0f50000f          	fence	iorw,ow
    80000c6c:	0804a02f          	amoswap.w	zero,zero,(s1)
    80000c70:	f8bff0ef          	jal	80000bfa <pop_off>
    80000c74:	60e2                	ld	ra,24(sp)
    80000c76:	6442                	ld	s0,16(sp)
    80000c78:	64a2                	ld	s1,8(sp)
    80000c7a:	6105                	add	sp,sp,32
    80000c7c:	8082                	ret
    80000c7e:	00006517          	auipc	a0,0x6
    80000c82:	41250513          	add	a0,a0,1042 # 80007090 <digits+0x78>
    80000c86:	b3dff0ef          	jal	800007c2 <panic>

0000000080000c8a <memset>:
    80000c8a:	1141                	add	sp,sp,-16
    80000c8c:	e422                	sd	s0,8(sp)
    80000c8e:	0800                	add	s0,sp,16
    80000c90:	ce09                	beqz	a2,80000caa <memset+0x20>
    80000c92:	87aa                	mv	a5,a0
    80000c94:	fff6071b          	addw	a4,a2,-1 # fff <_entry-0x7ffff001>
    80000c98:	1702                	sll	a4,a4,0x20
    80000c9a:	9301                	srl	a4,a4,0x20
    80000c9c:	0705                	add	a4,a4,1
    80000c9e:	972a                	add	a4,a4,a0
    80000ca0:	00b78023          	sb	a1,0(a5)
    80000ca4:	0785                	add	a5,a5,1
    80000ca6:	fee79de3          	bne	a5,a4,80000ca0 <memset+0x16>
    80000caa:	6422                	ld	s0,8(sp)
    80000cac:	0141                	add	sp,sp,16
    80000cae:	8082                	ret

0000000080000cb0 <memcmp>:
    80000cb0:	1141                	add	sp,sp,-16
    80000cb2:	e422                	sd	s0,8(sp)
    80000cb4:	0800                	add	s0,sp,16
    80000cb6:	ce15                	beqz	a2,80000cf2 <memcmp+0x42>
    80000cb8:	fff6069b          	addw	a3,a2,-1
    80000cbc:	00054783          	lbu	a5,0(a0)
    80000cc0:	0005c703          	lbu	a4,0(a1)
    80000cc4:	02e79063          	bne	a5,a4,80000ce4 <memcmp+0x34>
    80000cc8:	1682                	sll	a3,a3,0x20
    80000cca:	9281                	srl	a3,a3,0x20
    80000ccc:	0685                	add	a3,a3,1
    80000cce:	96aa                	add	a3,a3,a0
    80000cd0:	0505                	add	a0,a0,1
    80000cd2:	0585                	add	a1,a1,1
    80000cd4:	00d50d63          	beq	a0,a3,80000cee <memcmp+0x3e>
    80000cd8:	00054783          	lbu	a5,0(a0)
    80000cdc:	0005c703          	lbu	a4,0(a1)
    80000ce0:	fee788e3          	beq	a5,a4,80000cd0 <memcmp+0x20>
    80000ce4:	40e7853b          	subw	a0,a5,a4
    80000ce8:	6422                	ld	s0,8(sp)
    80000cea:	0141                	add	sp,sp,16
    80000cec:	8082                	ret
    80000cee:	4501                	li	a0,0
    80000cf0:	bfe5                	j	80000ce8 <memcmp+0x38>
    80000cf2:	4501                	li	a0,0
    80000cf4:	bfd5                	j	80000ce8 <memcmp+0x38>

0000000080000cf6 <memmove>:
    80000cf6:	1141                	add	sp,sp,-16
    80000cf8:	e422                	sd	s0,8(sp)
    80000cfa:	0800                	add	s0,sp,16
    80000cfc:	c215                	beqz	a2,80000d20 <memmove+0x2a>
    80000cfe:	02a5e463          	bltu	a1,a0,80000d26 <memmove+0x30>
    80000d02:	fff6079b          	addw	a5,a2,-1
    80000d06:	1782                	sll	a5,a5,0x20
    80000d08:	9381                	srl	a5,a5,0x20
    80000d0a:	0785                	add	a5,a5,1
    80000d0c:	97ae                	add	a5,a5,a1
    80000d0e:	872a                	mv	a4,a0
    80000d10:	0585                	add	a1,a1,1
    80000d12:	0705                	add	a4,a4,1
    80000d14:	fff5c683          	lbu	a3,-1(a1)
    80000d18:	fed70fa3          	sb	a3,-1(a4)
    80000d1c:	fef59ae3          	bne	a1,a5,80000d10 <memmove+0x1a>
    80000d20:	6422                	ld	s0,8(sp)
    80000d22:	0141                	add	sp,sp,16
    80000d24:	8082                	ret
    80000d26:	02061693          	sll	a3,a2,0x20
    80000d2a:	9281                	srl	a3,a3,0x20
    80000d2c:	00d58733          	add	a4,a1,a3
    80000d30:	fce579e3          	bgeu	a0,a4,80000d02 <memmove+0xc>
    80000d34:	96aa                	add	a3,a3,a0
    80000d36:	fff6079b          	addw	a5,a2,-1
    80000d3a:	1782                	sll	a5,a5,0x20
    80000d3c:	9381                	srl	a5,a5,0x20
    80000d3e:	fff7c793          	not	a5,a5
    80000d42:	97ba                	add	a5,a5,a4
    80000d44:	177d                	add	a4,a4,-1
    80000d46:	16fd                	add	a3,a3,-1
    80000d48:	00074603          	lbu	a2,0(a4)
    80000d4c:	00c68023          	sb	a2,0(a3)
    80000d50:	fee79ae3          	bne	a5,a4,80000d44 <memmove+0x4e>
    80000d54:	b7f1                	j	80000d20 <memmove+0x2a>

0000000080000d56 <memcpy>:
    80000d56:	1141                	add	sp,sp,-16
    80000d58:	e406                	sd	ra,8(sp)
    80000d5a:	e022                	sd	s0,0(sp)
    80000d5c:	0800                	add	s0,sp,16
    80000d5e:	f99ff0ef          	jal	80000cf6 <memmove>
    80000d62:	60a2                	ld	ra,8(sp)
    80000d64:	6402                	ld	s0,0(sp)
    80000d66:	0141                	add	sp,sp,16
    80000d68:	8082                	ret

0000000080000d6a <strncmp>:
    80000d6a:	1141                	add	sp,sp,-16
    80000d6c:	e422                	sd	s0,8(sp)
    80000d6e:	0800                	add	s0,sp,16
    80000d70:	c229                	beqz	a2,80000db2 <strncmp+0x48>
    80000d72:	00054783          	lbu	a5,0(a0)
    80000d76:	c795                	beqz	a5,80000da2 <strncmp+0x38>
    80000d78:	0005c703          	lbu	a4,0(a1)
    80000d7c:	02f71363          	bne	a4,a5,80000da2 <strncmp+0x38>
    80000d80:	fff6071b          	addw	a4,a2,-1
    80000d84:	1702                	sll	a4,a4,0x20
    80000d86:	9301                	srl	a4,a4,0x20
    80000d88:	0705                	add	a4,a4,1
    80000d8a:	972a                	add	a4,a4,a0
    80000d8c:	0505                	add	a0,a0,1
    80000d8e:	0585                	add	a1,a1,1
    80000d90:	02e50363          	beq	a0,a4,80000db6 <strncmp+0x4c>
    80000d94:	00054783          	lbu	a5,0(a0)
    80000d98:	c789                	beqz	a5,80000da2 <strncmp+0x38>
    80000d9a:	0005c683          	lbu	a3,0(a1)
    80000d9e:	fef687e3          	beq	a3,a5,80000d8c <strncmp+0x22>
    80000da2:	00054503          	lbu	a0,0(a0)
    80000da6:	0005c783          	lbu	a5,0(a1)
    80000daa:	9d1d                	subw	a0,a0,a5
    80000dac:	6422                	ld	s0,8(sp)
    80000dae:	0141                	add	sp,sp,16
    80000db0:	8082                	ret
    80000db2:	4501                	li	a0,0
    80000db4:	bfe5                	j	80000dac <strncmp+0x42>
    80000db6:	4501                	li	a0,0
    80000db8:	bfd5                	j	80000dac <strncmp+0x42>

0000000080000dba <strncpy>:
    80000dba:	1141                	add	sp,sp,-16
    80000dbc:	e422                	sd	s0,8(sp)
    80000dbe:	0800                	add	s0,sp,16
    80000dc0:	872a                	mv	a4,a0
    80000dc2:	a011                	j	80000dc6 <strncpy+0xc>
    80000dc4:	8642                	mv	a2,a6
    80000dc6:	fff6081b          	addw	a6,a2,-1
    80000dca:	00c05963          	blez	a2,80000ddc <strncpy+0x22>
    80000dce:	0705                	add	a4,a4,1
    80000dd0:	0005c783          	lbu	a5,0(a1)
    80000dd4:	fef70fa3          	sb	a5,-1(a4)
    80000dd8:	0585                	add	a1,a1,1
    80000dda:	f7ed                	bnez	a5,80000dc4 <strncpy+0xa>
    80000ddc:	86ba                	mv	a3,a4
    80000dde:	01005b63          	blez	a6,80000df4 <strncpy+0x3a>
    80000de2:	0685                	add	a3,a3,1
    80000de4:	fe068fa3          	sb	zero,-1(a3)
    80000de8:	fff6c793          	not	a5,a3
    80000dec:	9fb9                	addw	a5,a5,a4
    80000dee:	9fb1                	addw	a5,a5,a2
    80000df0:	fef049e3          	bgtz	a5,80000de2 <strncpy+0x28>
    80000df4:	6422                	ld	s0,8(sp)
    80000df6:	0141                	add	sp,sp,16
    80000df8:	8082                	ret

0000000080000dfa <safestrcpy>:
    80000dfa:	1141                	add	sp,sp,-16
    80000dfc:	e422                	sd	s0,8(sp)
    80000dfe:	0800                	add	s0,sp,16
    80000e00:	02c05363          	blez	a2,80000e26 <safestrcpy+0x2c>
    80000e04:	fff6069b          	addw	a3,a2,-1
    80000e08:	1682                	sll	a3,a3,0x20
    80000e0a:	9281                	srl	a3,a3,0x20
    80000e0c:	96ae                	add	a3,a3,a1
    80000e0e:	87aa                	mv	a5,a0
    80000e10:	00d58963          	beq	a1,a3,80000e22 <safestrcpy+0x28>
    80000e14:	0585                	add	a1,a1,1
    80000e16:	0785                	add	a5,a5,1
    80000e18:	fff5c703          	lbu	a4,-1(a1)
    80000e1c:	fee78fa3          	sb	a4,-1(a5)
    80000e20:	fb65                	bnez	a4,80000e10 <safestrcpy+0x16>
    80000e22:	00078023          	sb	zero,0(a5)
    80000e26:	6422                	ld	s0,8(sp)
    80000e28:	0141                	add	sp,sp,16
    80000e2a:	8082                	ret

0000000080000e2c <strlen>:
    80000e2c:	1141                	add	sp,sp,-16
    80000e2e:	e422                	sd	s0,8(sp)
    80000e30:	0800                	add	s0,sp,16
    80000e32:	00054783          	lbu	a5,0(a0)
    80000e36:	cf91                	beqz	a5,80000e52 <strlen+0x26>
    80000e38:	0505                	add	a0,a0,1
    80000e3a:	87aa                	mv	a5,a0
    80000e3c:	4685                	li	a3,1
    80000e3e:	9e89                	subw	a3,a3,a0
    80000e40:	00f6853b          	addw	a0,a3,a5
    80000e44:	0785                	add	a5,a5,1
    80000e46:	fff7c703          	lbu	a4,-1(a5)
    80000e4a:	fb7d                	bnez	a4,80000e40 <strlen+0x14>
    80000e4c:	6422                	ld	s0,8(sp)
    80000e4e:	0141                	add	sp,sp,16
    80000e50:	8082                	ret
    80000e52:	4501                	li	a0,0
    80000e54:	bfe5                	j	80000e4c <strlen+0x20>

0000000080000e56 <main>:
    80000e56:	1141                	add	sp,sp,-16
    80000e58:	e406                	sd	ra,8(sp)
    80000e5a:	e022                	sd	s0,0(sp)
    80000e5c:	0800                	add	s0,sp,16
    80000e5e:	203000ef          	jal	80001860 <cpuid>
    80000e62:	00007717          	auipc	a4,0x7
    80000e66:	9de70713          	add	a4,a4,-1570 # 80007840 <started>
    80000e6a:	c51d                	beqz	a0,80000e98 <main+0x42>
    80000e6c:	431c                	lw	a5,0(a4)
    80000e6e:	2781                	sext.w	a5,a5
    80000e70:	dff5                	beqz	a5,80000e6c <main+0x16>
    80000e72:	0ff0000f          	fence
    80000e76:	1eb000ef          	jal	80001860 <cpuid>
    80000e7a:	85aa                	mv	a1,a0
    80000e7c:	00006517          	auipc	a0,0x6
    80000e80:	23450513          	add	a0,a0,564 # 800070b0 <digits+0x98>
    80000e84:	e70ff0ef          	jal	800004f4 <printf>
    80000e88:	080000ef          	jal	80000f08 <kvminithart>
    80000e8c:	52c010ef          	jal	800023b8 <trapinithart>
    80000e90:	4c4040ef          	jal	80005354 <plicinithart>
    80000e94:	667000ef          	jal	80001cfa <scheduler>
    80000e98:	d80ff0ef          	jal	80000418 <consoleinit>
    80000e9c:	963ff0ef          	jal	800007fe <printfinit>
    80000ea0:	00006517          	auipc	a0,0x6
    80000ea4:	22050513          	add	a0,a0,544 # 800070c0 <digits+0xa8>
    80000ea8:	e4cff0ef          	jal	800004f4 <printf>
    80000eac:	00006517          	auipc	a0,0x6
    80000eb0:	1ec50513          	add	a0,a0,492 # 80007098 <digits+0x80>
    80000eb4:	e40ff0ef          	jal	800004f4 <printf>
    80000eb8:	00006517          	auipc	a0,0x6
    80000ebc:	20850513          	add	a0,a0,520 # 800070c0 <digits+0xa8>
    80000ec0:	e34ff0ef          	jal	800004f4 <printf>
    80000ec4:	befff0ef          	jal	80000ab2 <kinit>
    80000ec8:	2cc000ef          	jal	80001194 <kvminit>
    80000ecc:	03c000ef          	jal	80000f08 <kvminithart>
    80000ed0:	0e9000ef          	jal	800017b8 <procinit>
    80000ed4:	4c0010ef          	jal	80002394 <trapinit>
    80000ed8:	4e0010ef          	jal	800023b8 <trapinithart>
    80000edc:	462040ef          	jal	8000533e <plicinit>
    80000ee0:	474040ef          	jal	80005354 <plicinithart>
    80000ee4:	369010ef          	jal	80002a4c <binit>
    80000ee8:	10e020ef          	jal	80002ff6 <iinit>
    80000eec:	01e030ef          	jal	80003f0a <fileinit>
    80000ef0:	554040ef          	jal	80005444 <virtio_disk_init>
    80000ef4:	461000ef          	jal	80001b54 <userinit>
    80000ef8:	0ff0000f          	fence
    80000efc:	4785                	li	a5,1
    80000efe:	00007717          	auipc	a4,0x7
    80000f02:	94f72123          	sw	a5,-1726(a4) # 80007840 <started>
    80000f06:	b779                	j	80000e94 <main+0x3e>

0000000080000f08 <kvminithart>:
    80000f08:	1141                	add	sp,sp,-16
    80000f0a:	e422                	sd	s0,8(sp)
    80000f0c:	0800                	add	s0,sp,16
    80000f0e:	12000073          	sfence.vma
    80000f12:	00007797          	auipc	a5,0x7
    80000f16:	93678793          	add	a5,a5,-1738 # 80007848 <kernel_pagetable>
    80000f1a:	639c                	ld	a5,0(a5)
    80000f1c:	83b1                	srl	a5,a5,0xc
    80000f1e:	577d                	li	a4,-1
    80000f20:	177e                	sll	a4,a4,0x3f
    80000f22:	8fd9                	or	a5,a5,a4
    80000f24:	18079073          	csrw	satp,a5
    80000f28:	12000073          	sfence.vma
    80000f2c:	6422                	ld	s0,8(sp)
    80000f2e:	0141                	add	sp,sp,16
    80000f30:	8082                	ret

0000000080000f32 <walk>:
    80000f32:	7139                	add	sp,sp,-64
    80000f34:	fc06                	sd	ra,56(sp)
    80000f36:	f822                	sd	s0,48(sp)
    80000f38:	f426                	sd	s1,40(sp)
    80000f3a:	f04a                	sd	s2,32(sp)
    80000f3c:	ec4e                	sd	s3,24(sp)
    80000f3e:	e852                	sd	s4,16(sp)
    80000f40:	e456                	sd	s5,8(sp)
    80000f42:	e05a                	sd	s6,0(sp)
    80000f44:	0080                	add	s0,sp,64
    80000f46:	84aa                	mv	s1,a0
    80000f48:	89ae                	mv	s3,a1
    80000f4a:	8b32                	mv	s6,a2
    80000f4c:	57fd                	li	a5,-1
    80000f4e:	83e9                	srl	a5,a5,0x1a
    80000f50:	4a79                	li	s4,30
    80000f52:	4ab1                	li	s5,12
    80000f54:	02b7fc63          	bgeu	a5,a1,80000f8c <walk+0x5a>
    80000f58:	00006517          	auipc	a0,0x6
    80000f5c:	17050513          	add	a0,a0,368 # 800070c8 <digits+0xb0>
    80000f60:	863ff0ef          	jal	800007c2 <panic>
    80000f64:	060b0263          	beqz	s6,80000fc8 <walk+0x96>
    80000f68:	b7fff0ef          	jal	80000ae6 <kalloc>
    80000f6c:	84aa                	mv	s1,a0
    80000f6e:	c139                	beqz	a0,80000fb4 <walk+0x82>
    80000f70:	6605                	lui	a2,0x1
    80000f72:	4581                	li	a1,0
    80000f74:	d17ff0ef          	jal	80000c8a <memset>
    80000f78:	00c4d793          	srl	a5,s1,0xc
    80000f7c:	07aa                	sll	a5,a5,0xa
    80000f7e:	0017e793          	or	a5,a5,1
    80000f82:	00f93023          	sd	a5,0(s2)
    80000f86:	3a5d                	addw	s4,s4,-9 # ffffffffffffeff7 <end+0xffffffff7ffde48f>
    80000f88:	035a0063          	beq	s4,s5,80000fa8 <walk+0x76>
    80000f8c:	0149d933          	srl	s2,s3,s4
    80000f90:	1ff97913          	and	s2,s2,511
    80000f94:	090e                	sll	s2,s2,0x3
    80000f96:	9926                	add	s2,s2,s1
    80000f98:	00093483          	ld	s1,0(s2)
    80000f9c:	0014f793          	and	a5,s1,1
    80000fa0:	d3f1                	beqz	a5,80000f64 <walk+0x32>
    80000fa2:	80a9                	srl	s1,s1,0xa
    80000fa4:	04b2                	sll	s1,s1,0xc
    80000fa6:	b7c5                	j	80000f86 <walk+0x54>
    80000fa8:	00c9d513          	srl	a0,s3,0xc
    80000fac:	1ff57513          	and	a0,a0,511
    80000fb0:	050e                	sll	a0,a0,0x3
    80000fb2:	9526                	add	a0,a0,s1
    80000fb4:	70e2                	ld	ra,56(sp)
    80000fb6:	7442                	ld	s0,48(sp)
    80000fb8:	74a2                	ld	s1,40(sp)
    80000fba:	7902                	ld	s2,32(sp)
    80000fbc:	69e2                	ld	s3,24(sp)
    80000fbe:	6a42                	ld	s4,16(sp)
    80000fc0:	6aa2                	ld	s5,8(sp)
    80000fc2:	6b02                	ld	s6,0(sp)
    80000fc4:	6121                	add	sp,sp,64
    80000fc6:	8082                	ret
    80000fc8:	4501                	li	a0,0
    80000fca:	b7ed                	j	80000fb4 <walk+0x82>

0000000080000fcc <walkaddr>:
    80000fcc:	57fd                	li	a5,-1
    80000fce:	83e9                	srl	a5,a5,0x1a
    80000fd0:	00b7f463          	bgeu	a5,a1,80000fd8 <walkaddr+0xc>
    80000fd4:	4501                	li	a0,0
    80000fd6:	8082                	ret
    80000fd8:	1141                	add	sp,sp,-16
    80000fda:	e406                	sd	ra,8(sp)
    80000fdc:	e022                	sd	s0,0(sp)
    80000fde:	0800                	add	s0,sp,16
    80000fe0:	4601                	li	a2,0
    80000fe2:	f51ff0ef          	jal	80000f32 <walk>
    80000fe6:	c105                	beqz	a0,80001006 <walkaddr+0x3a>
    80000fe8:	611c                	ld	a5,0(a0)
    80000fea:	0117f693          	and	a3,a5,17
    80000fee:	4745                	li	a4,17
    80000ff0:	4501                	li	a0,0
    80000ff2:	00e68663          	beq	a3,a4,80000ffe <walkaddr+0x32>
    80000ff6:	60a2                	ld	ra,8(sp)
    80000ff8:	6402                	ld	s0,0(sp)
    80000ffa:	0141                	add	sp,sp,16
    80000ffc:	8082                	ret
    80000ffe:	00a7d513          	srl	a0,a5,0xa
    80001002:	0532                	sll	a0,a0,0xc
    80001004:	bfcd                	j	80000ff6 <walkaddr+0x2a>
    80001006:	4501                	li	a0,0
    80001008:	b7fd                	j	80000ff6 <walkaddr+0x2a>

000000008000100a <mappages>:
    8000100a:	715d                	add	sp,sp,-80
    8000100c:	e486                	sd	ra,72(sp)
    8000100e:	e0a2                	sd	s0,64(sp)
    80001010:	fc26                	sd	s1,56(sp)
    80001012:	f84a                	sd	s2,48(sp)
    80001014:	f44e                	sd	s3,40(sp)
    80001016:	f052                	sd	s4,32(sp)
    80001018:	ec56                	sd	s5,24(sp)
    8000101a:	e85a                	sd	s6,16(sp)
    8000101c:	e45e                	sd	s7,8(sp)
    8000101e:	0880                	add	s0,sp,80
    80001020:	03459793          	sll	a5,a1,0x34
    80001024:	e385                	bnez	a5,80001044 <mappages+0x3a>
    80001026:	8aaa                	mv	s5,a0
    80001028:	8b3a                	mv	s6,a4
    8000102a:	03461793          	sll	a5,a2,0x34
    8000102e:	e38d                	bnez	a5,80001050 <mappages+0x46>
    80001030:	c615                	beqz	a2,8000105c <mappages+0x52>
    80001032:	79fd                	lui	s3,0xfffff
    80001034:	964e                	add	a2,a2,s3
    80001036:	00b609b3          	add	s3,a2,a1
    8000103a:	892e                	mv	s2,a1
    8000103c:	40b68a33          	sub	s4,a3,a1
    80001040:	6b85                	lui	s7,0x1
    80001042:	a815                	j	80001076 <mappages+0x6c>
    80001044:	00006517          	auipc	a0,0x6
    80001048:	08c50513          	add	a0,a0,140 # 800070d0 <digits+0xb8>
    8000104c:	f76ff0ef          	jal	800007c2 <panic>
    80001050:	00006517          	auipc	a0,0x6
    80001054:	0a050513          	add	a0,a0,160 # 800070f0 <digits+0xd8>
    80001058:	f6aff0ef          	jal	800007c2 <panic>
    8000105c:	00006517          	auipc	a0,0x6
    80001060:	0b450513          	add	a0,a0,180 # 80007110 <digits+0xf8>
    80001064:	f5eff0ef          	jal	800007c2 <panic>
    80001068:	00006517          	auipc	a0,0x6
    8000106c:	0b850513          	add	a0,a0,184 # 80007120 <digits+0x108>
    80001070:	f52ff0ef          	jal	800007c2 <panic>
    80001074:	995e                	add	s2,s2,s7
    80001076:	012a04b3          	add	s1,s4,s2
    8000107a:	4605                	li	a2,1
    8000107c:	85ca                	mv	a1,s2
    8000107e:	8556                	mv	a0,s5
    80001080:	eb3ff0ef          	jal	80000f32 <walk>
    80001084:	cd19                	beqz	a0,800010a2 <mappages+0x98>
    80001086:	611c                	ld	a5,0(a0)
    80001088:	8b85                	and	a5,a5,1
    8000108a:	fff9                	bnez	a5,80001068 <mappages+0x5e>
    8000108c:	80b1                	srl	s1,s1,0xc
    8000108e:	04aa                	sll	s1,s1,0xa
    80001090:	0164e4b3          	or	s1,s1,s6
    80001094:	0014e493          	or	s1,s1,1
    80001098:	e104                	sd	s1,0(a0)
    8000109a:	fd391de3          	bne	s2,s3,80001074 <mappages+0x6a>
    8000109e:	4501                	li	a0,0
    800010a0:	a011                	j	800010a4 <mappages+0x9a>
    800010a2:	557d                	li	a0,-1
    800010a4:	60a6                	ld	ra,72(sp)
    800010a6:	6406                	ld	s0,64(sp)
    800010a8:	74e2                	ld	s1,56(sp)
    800010aa:	7942                	ld	s2,48(sp)
    800010ac:	79a2                	ld	s3,40(sp)
    800010ae:	7a02                	ld	s4,32(sp)
    800010b0:	6ae2                	ld	s5,24(sp)
    800010b2:	6b42                	ld	s6,16(sp)
    800010b4:	6ba2                	ld	s7,8(sp)
    800010b6:	6161                	add	sp,sp,80
    800010b8:	8082                	ret

00000000800010ba <kvmmap>:
    800010ba:	1141                	add	sp,sp,-16
    800010bc:	e406                	sd	ra,8(sp)
    800010be:	e022                	sd	s0,0(sp)
    800010c0:	0800                	add	s0,sp,16
    800010c2:	87b6                	mv	a5,a3
    800010c4:	86b2                	mv	a3,a2
    800010c6:	863e                	mv	a2,a5
    800010c8:	f43ff0ef          	jal	8000100a <mappages>
    800010cc:	e509                	bnez	a0,800010d6 <kvmmap+0x1c>
    800010ce:	60a2                	ld	ra,8(sp)
    800010d0:	6402                	ld	s0,0(sp)
    800010d2:	0141                	add	sp,sp,16
    800010d4:	8082                	ret
    800010d6:	00006517          	auipc	a0,0x6
    800010da:	05a50513          	add	a0,a0,90 # 80007130 <digits+0x118>
    800010de:	ee4ff0ef          	jal	800007c2 <panic>

00000000800010e2 <kvmmake>:
    800010e2:	1101                	add	sp,sp,-32
    800010e4:	ec06                	sd	ra,24(sp)
    800010e6:	e822                	sd	s0,16(sp)
    800010e8:	e426                	sd	s1,8(sp)
    800010ea:	e04a                	sd	s2,0(sp)
    800010ec:	1000                	add	s0,sp,32
    800010ee:	9f9ff0ef          	jal	80000ae6 <kalloc>
    800010f2:	84aa                	mv	s1,a0
    800010f4:	6605                	lui	a2,0x1
    800010f6:	4581                	li	a1,0
    800010f8:	b93ff0ef          	jal	80000c8a <memset>
    800010fc:	4719                	li	a4,6
    800010fe:	6685                	lui	a3,0x1
    80001100:	10000637          	lui	a2,0x10000
    80001104:	100005b7          	lui	a1,0x10000
    80001108:	8526                	mv	a0,s1
    8000110a:	fb1ff0ef          	jal	800010ba <kvmmap>
    8000110e:	4719                	li	a4,6
    80001110:	6685                	lui	a3,0x1
    80001112:	10001637          	lui	a2,0x10001
    80001116:	100015b7          	lui	a1,0x10001
    8000111a:	8526                	mv	a0,s1
    8000111c:	f9fff0ef          	jal	800010ba <kvmmap>
    80001120:	4719                	li	a4,6
    80001122:	040006b7          	lui	a3,0x4000
    80001126:	0c000637          	lui	a2,0xc000
    8000112a:	0c0005b7          	lui	a1,0xc000
    8000112e:	8526                	mv	a0,s1
    80001130:	f8bff0ef          	jal	800010ba <kvmmap>
    80001134:	00006917          	auipc	s2,0x6
    80001138:	ecc90913          	add	s2,s2,-308 # 80007000 <etext>
    8000113c:	4729                	li	a4,10
    8000113e:	80006697          	auipc	a3,0x80006
    80001142:	ec268693          	add	a3,a3,-318 # 7000 <_entry-0x7fff9000>
    80001146:	4605                	li	a2,1
    80001148:	067e                	sll	a2,a2,0x1f
    8000114a:	85b2                	mv	a1,a2
    8000114c:	8526                	mv	a0,s1
    8000114e:	f6dff0ef          	jal	800010ba <kvmmap>
    80001152:	4719                	li	a4,6
    80001154:	46c5                	li	a3,17
    80001156:	06ee                	sll	a3,a3,0x1b
    80001158:	412686b3          	sub	a3,a3,s2
    8000115c:	864a                	mv	a2,s2
    8000115e:	85ca                	mv	a1,s2
    80001160:	8526                	mv	a0,s1
    80001162:	f59ff0ef          	jal	800010ba <kvmmap>
    80001166:	4729                	li	a4,10
    80001168:	6685                	lui	a3,0x1
    8000116a:	00005617          	auipc	a2,0x5
    8000116e:	e9660613          	add	a2,a2,-362 # 80006000 <_trampoline>
    80001172:	040005b7          	lui	a1,0x4000
    80001176:	15fd                	add	a1,a1,-1 # 3ffffff <_entry-0x7c000001>
    80001178:	05b2                	sll	a1,a1,0xc
    8000117a:	8526                	mv	a0,s1
    8000117c:	f3fff0ef          	jal	800010ba <kvmmap>
    80001180:	8526                	mv	a0,s1
    80001182:	5ac000ef          	jal	8000172e <proc_mapstacks>
    80001186:	8526                	mv	a0,s1
    80001188:	60e2                	ld	ra,24(sp)
    8000118a:	6442                	ld	s0,16(sp)
    8000118c:	64a2                	ld	s1,8(sp)
    8000118e:	6902                	ld	s2,0(sp)
    80001190:	6105                	add	sp,sp,32
    80001192:	8082                	ret

0000000080001194 <kvminit>:
    80001194:	1141                	add	sp,sp,-16
    80001196:	e406                	sd	ra,8(sp)
    80001198:	e022                	sd	s0,0(sp)
    8000119a:	0800                	add	s0,sp,16
    8000119c:	f47ff0ef          	jal	800010e2 <kvmmake>
    800011a0:	00006797          	auipc	a5,0x6
    800011a4:	6aa7b423          	sd	a0,1704(a5) # 80007848 <kernel_pagetable>
    800011a8:	60a2                	ld	ra,8(sp)
    800011aa:	6402                	ld	s0,0(sp)
    800011ac:	0141                	add	sp,sp,16
    800011ae:	8082                	ret

00000000800011b0 <uvmcreate>:
    800011b0:	1101                	add	sp,sp,-32
    800011b2:	ec06                	sd	ra,24(sp)
    800011b4:	e822                	sd	s0,16(sp)
    800011b6:	e426                	sd	s1,8(sp)
    800011b8:	1000                	add	s0,sp,32
    800011ba:	92dff0ef          	jal	80000ae6 <kalloc>
    800011be:	84aa                	mv	s1,a0
    800011c0:	c509                	beqz	a0,800011ca <uvmcreate+0x1a>
    800011c2:	6605                	lui	a2,0x1
    800011c4:	4581                	li	a1,0
    800011c6:	ac5ff0ef          	jal	80000c8a <memset>
    800011ca:	8526                	mv	a0,s1
    800011cc:	60e2                	ld	ra,24(sp)
    800011ce:	6442                	ld	s0,16(sp)
    800011d0:	64a2                	ld	s1,8(sp)
    800011d2:	6105                	add	sp,sp,32
    800011d4:	8082                	ret

00000000800011d6 <uvmunmap>:
    800011d6:	7139                	add	sp,sp,-64
    800011d8:	fc06                	sd	ra,56(sp)
    800011da:	f822                	sd	s0,48(sp)
    800011dc:	f426                	sd	s1,40(sp)
    800011de:	f04a                	sd	s2,32(sp)
    800011e0:	ec4e                	sd	s3,24(sp)
    800011e2:	e852                	sd	s4,16(sp)
    800011e4:	e456                	sd	s5,8(sp)
    800011e6:	e05a                	sd	s6,0(sp)
    800011e8:	0080                	add	s0,sp,64
    800011ea:	03459793          	sll	a5,a1,0x34
    800011ee:	e785                	bnez	a5,80001216 <uvmunmap+0x40>
    800011f0:	8aaa                	mv	s5,a0
    800011f2:	84ae                	mv	s1,a1
    800011f4:	8b36                	mv	s6,a3
    800011f6:	0632                	sll	a2,a2,0xc
    800011f8:	00b609b3          	add	s3,a2,a1
    800011fc:	6a05                	lui	s4,0x1
    800011fe:	0335eb63          	bltu	a1,s3,80001234 <uvmunmap+0x5e>
    80001202:	70e2                	ld	ra,56(sp)
    80001204:	7442                	ld	s0,48(sp)
    80001206:	74a2                	ld	s1,40(sp)
    80001208:	7902                	ld	s2,32(sp)
    8000120a:	69e2                	ld	s3,24(sp)
    8000120c:	6a42                	ld	s4,16(sp)
    8000120e:	6aa2                	ld	s5,8(sp)
    80001210:	6b02                	ld	s6,0(sp)
    80001212:	6121                	add	sp,sp,64
    80001214:	8082                	ret
    80001216:	00006517          	auipc	a0,0x6
    8000121a:	f2250513          	add	a0,a0,-222 # 80007138 <digits+0x120>
    8000121e:	da4ff0ef          	jal	800007c2 <panic>
    80001222:	8129                	srl	a0,a0,0xa
    80001224:	0532                	sll	a0,a0,0xc
    80001226:	fdeff0ef          	jal	80000a04 <kfree>
    8000122a:	00093023          	sd	zero,0(s2)
    8000122e:	94d2                	add	s1,s1,s4
    80001230:	fd34f9e3          	bgeu	s1,s3,80001202 <uvmunmap+0x2c>
    80001234:	4601                	li	a2,0
    80001236:	85a6                	mv	a1,s1
    80001238:	8556                	mv	a0,s5
    8000123a:	cf9ff0ef          	jal	80000f32 <walk>
    8000123e:	892a                	mv	s2,a0
    80001240:	d57d                	beqz	a0,8000122e <uvmunmap+0x58>
    80001242:	6108                	ld	a0,0(a0)
    80001244:	00157793          	and	a5,a0,1
    80001248:	d3fd                	beqz	a5,8000122e <uvmunmap+0x58>
    8000124a:	fe0b00e3          	beqz	s6,8000122a <uvmunmap+0x54>
    8000124e:	bfd1                	j	80001222 <uvmunmap+0x4c>

0000000080001250 <uvmdealloc>:
    80001250:	1101                	add	sp,sp,-32
    80001252:	ec06                	sd	ra,24(sp)
    80001254:	e822                	sd	s0,16(sp)
    80001256:	e426                	sd	s1,8(sp)
    80001258:	1000                	add	s0,sp,32
    8000125a:	84ae                	mv	s1,a1
    8000125c:	00b67d63          	bgeu	a2,a1,80001276 <uvmdealloc+0x26>
    80001260:	84b2                	mv	s1,a2
    80001262:	6605                	lui	a2,0x1
    80001264:	167d                	add	a2,a2,-1 # fff <_entry-0x7ffff001>
    80001266:	00c487b3          	add	a5,s1,a2
    8000126a:	777d                	lui	a4,0xfffff
    8000126c:	8ff9                	and	a5,a5,a4
    8000126e:	962e                	add	a2,a2,a1
    80001270:	8e79                	and	a2,a2,a4
    80001272:	00c7e863          	bltu	a5,a2,80001282 <uvmdealloc+0x32>
    80001276:	8526                	mv	a0,s1
    80001278:	60e2                	ld	ra,24(sp)
    8000127a:	6442                	ld	s0,16(sp)
    8000127c:	64a2                	ld	s1,8(sp)
    8000127e:	6105                	add	sp,sp,32
    80001280:	8082                	ret
    80001282:	8e1d                	sub	a2,a2,a5
    80001284:	8231                	srl	a2,a2,0xc
    80001286:	4685                	li	a3,1
    80001288:	2601                	sext.w	a2,a2
    8000128a:	85be                	mv	a1,a5
    8000128c:	f4bff0ef          	jal	800011d6 <uvmunmap>
    80001290:	b7dd                	j	80001276 <uvmdealloc+0x26>

0000000080001292 <uvmalloc>:
    80001292:	08b66963          	bltu	a2,a1,80001324 <uvmalloc+0x92>
    80001296:	7139                	add	sp,sp,-64
    80001298:	fc06                	sd	ra,56(sp)
    8000129a:	f822                	sd	s0,48(sp)
    8000129c:	f426                	sd	s1,40(sp)
    8000129e:	f04a                	sd	s2,32(sp)
    800012a0:	ec4e                	sd	s3,24(sp)
    800012a2:	e852                	sd	s4,16(sp)
    800012a4:	e456                	sd	s5,8(sp)
    800012a6:	e05a                	sd	s6,0(sp)
    800012a8:	0080                	add	s0,sp,64
    800012aa:	6a85                	lui	s5,0x1
    800012ac:	1afd                	add	s5,s5,-1 # fff <_entry-0x7ffff001>
    800012ae:	95d6                	add	a1,a1,s5
    800012b0:	7afd                	lui	s5,0xfffff
    800012b2:	0155fab3          	and	s5,a1,s5
    800012b6:	06caf963          	bgeu	s5,a2,80001328 <uvmalloc+0x96>
    800012ba:	89b2                	mv	s3,a2
    800012bc:	8b2a                	mv	s6,a0
    800012be:	8956                	mv	s2,s5
    800012c0:	0126ea13          	or	s4,a3,18
    800012c4:	823ff0ef          	jal	80000ae6 <kalloc>
    800012c8:	84aa                	mv	s1,a0
    800012ca:	c11d                	beqz	a0,800012f0 <uvmalloc+0x5e>
    800012cc:	6605                	lui	a2,0x1
    800012ce:	4581                	li	a1,0
    800012d0:	9bbff0ef          	jal	80000c8a <memset>
    800012d4:	8752                	mv	a4,s4
    800012d6:	86a6                	mv	a3,s1
    800012d8:	6605                	lui	a2,0x1
    800012da:	85ca                	mv	a1,s2
    800012dc:	855a                	mv	a0,s6
    800012de:	d2dff0ef          	jal	8000100a <mappages>
    800012e2:	e51d                	bnez	a0,80001310 <uvmalloc+0x7e>
    800012e4:	6785                	lui	a5,0x1
    800012e6:	993e                	add	s2,s2,a5
    800012e8:	fd396ee3          	bltu	s2,s3,800012c4 <uvmalloc+0x32>
    800012ec:	854e                	mv	a0,s3
    800012ee:	a039                	j	800012fc <uvmalloc+0x6a>
    800012f0:	8656                	mv	a2,s5
    800012f2:	85ca                	mv	a1,s2
    800012f4:	855a                	mv	a0,s6
    800012f6:	f5bff0ef          	jal	80001250 <uvmdealloc>
    800012fa:	4501                	li	a0,0
    800012fc:	70e2                	ld	ra,56(sp)
    800012fe:	7442                	ld	s0,48(sp)
    80001300:	74a2                	ld	s1,40(sp)
    80001302:	7902                	ld	s2,32(sp)
    80001304:	69e2                	ld	s3,24(sp)
    80001306:	6a42                	ld	s4,16(sp)
    80001308:	6aa2                	ld	s5,8(sp)
    8000130a:	6b02                	ld	s6,0(sp)
    8000130c:	6121                	add	sp,sp,64
    8000130e:	8082                	ret
    80001310:	8526                	mv	a0,s1
    80001312:	ef2ff0ef          	jal	80000a04 <kfree>
    80001316:	8656                	mv	a2,s5
    80001318:	85ca                	mv	a1,s2
    8000131a:	855a                	mv	a0,s6
    8000131c:	f35ff0ef          	jal	80001250 <uvmdealloc>
    80001320:	4501                	li	a0,0
    80001322:	bfe9                	j	800012fc <uvmalloc+0x6a>
    80001324:	852e                	mv	a0,a1
    80001326:	8082                	ret
    80001328:	8532                	mv	a0,a2
    8000132a:	bfc9                	j	800012fc <uvmalloc+0x6a>

000000008000132c <freewalk>:
    8000132c:	7179                	add	sp,sp,-48
    8000132e:	f406                	sd	ra,40(sp)
    80001330:	f022                	sd	s0,32(sp)
    80001332:	ec26                	sd	s1,24(sp)
    80001334:	e84a                	sd	s2,16(sp)
    80001336:	e44e                	sd	s3,8(sp)
    80001338:	e052                	sd	s4,0(sp)
    8000133a:	1800                	add	s0,sp,48
    8000133c:	8a2a                	mv	s4,a0
    8000133e:	84aa                	mv	s1,a0
    80001340:	6905                	lui	s2,0x1
    80001342:	992a                	add	s2,s2,a0
    80001344:	4985                	li	s3,1
    80001346:	a811                	j	8000135a <freewalk+0x2e>
    80001348:	8129                	srl	a0,a0,0xa
    8000134a:	0532                	sll	a0,a0,0xc
    8000134c:	fe1ff0ef          	jal	8000132c <freewalk>
    80001350:	0004b023          	sd	zero,0(s1)
    80001354:	04a1                	add	s1,s1,8
    80001356:	01248f63          	beq	s1,s2,80001374 <freewalk+0x48>
    8000135a:	6088                	ld	a0,0(s1)
    8000135c:	00f57793          	and	a5,a0,15
    80001360:	ff3784e3          	beq	a5,s3,80001348 <freewalk+0x1c>
    80001364:	8905                	and	a0,a0,1
    80001366:	d57d                	beqz	a0,80001354 <freewalk+0x28>
    80001368:	00006517          	auipc	a0,0x6
    8000136c:	de850513          	add	a0,a0,-536 # 80007150 <digits+0x138>
    80001370:	c52ff0ef          	jal	800007c2 <panic>
    80001374:	8552                	mv	a0,s4
    80001376:	e8eff0ef          	jal	80000a04 <kfree>
    8000137a:	70a2                	ld	ra,40(sp)
    8000137c:	7402                	ld	s0,32(sp)
    8000137e:	64e2                	ld	s1,24(sp)
    80001380:	6942                	ld	s2,16(sp)
    80001382:	69a2                	ld	s3,8(sp)
    80001384:	6a02                	ld	s4,0(sp)
    80001386:	6145                	add	sp,sp,48
    80001388:	8082                	ret

000000008000138a <uvmfree>:
    8000138a:	1101                	add	sp,sp,-32
    8000138c:	ec06                	sd	ra,24(sp)
    8000138e:	e822                	sd	s0,16(sp)
    80001390:	e426                	sd	s1,8(sp)
    80001392:	1000                	add	s0,sp,32
    80001394:	84aa                	mv	s1,a0
    80001396:	e989                	bnez	a1,800013a8 <uvmfree+0x1e>
    80001398:	8526                	mv	a0,s1
    8000139a:	f93ff0ef          	jal	8000132c <freewalk>
    8000139e:	60e2                	ld	ra,24(sp)
    800013a0:	6442                	ld	s0,16(sp)
    800013a2:	64a2                	ld	s1,8(sp)
    800013a4:	6105                	add	sp,sp,32
    800013a6:	8082                	ret
    800013a8:	6605                	lui	a2,0x1
    800013aa:	167d                	add	a2,a2,-1 # fff <_entry-0x7ffff001>
    800013ac:	962e                	add	a2,a2,a1
    800013ae:	4685                	li	a3,1
    800013b0:	8231                	srl	a2,a2,0xc
    800013b2:	4581                	li	a1,0
    800013b4:	e23ff0ef          	jal	800011d6 <uvmunmap>
    800013b8:	b7c5                	j	80001398 <uvmfree+0xe>

00000000800013ba <uvmcopy>:
    800013ba:	ce49                	beqz	a2,80001454 <uvmcopy+0x9a>
    800013bc:	715d                	add	sp,sp,-80
    800013be:	e486                	sd	ra,72(sp)
    800013c0:	e0a2                	sd	s0,64(sp)
    800013c2:	fc26                	sd	s1,56(sp)
    800013c4:	f84a                	sd	s2,48(sp)
    800013c6:	f44e                	sd	s3,40(sp)
    800013c8:	f052                	sd	s4,32(sp)
    800013ca:	ec56                	sd	s5,24(sp)
    800013cc:	e85a                	sd	s6,16(sp)
    800013ce:	e45e                	sd	s7,8(sp)
    800013d0:	0880                	add	s0,sp,80
    800013d2:	8a32                	mv	s4,a2
    800013d4:	8b2e                	mv	s6,a1
    800013d6:	8aaa                	mv	s5,a0
    800013d8:	4481                	li	s1,0
    800013da:	a029                	j	800013e4 <uvmcopy+0x2a>
    800013dc:	6785                	lui	a5,0x1
    800013de:	94be                	add	s1,s1,a5
    800013e0:	0544fe63          	bgeu	s1,s4,8000143c <uvmcopy+0x82>
    800013e4:	4601                	li	a2,0
    800013e6:	85a6                	mv	a1,s1
    800013e8:	8556                	mv	a0,s5
    800013ea:	b49ff0ef          	jal	80000f32 <walk>
    800013ee:	d57d                	beqz	a0,800013dc <uvmcopy+0x22>
    800013f0:	6118                	ld	a4,0(a0)
    800013f2:	00177793          	and	a5,a4,1
    800013f6:	d3fd                	beqz	a5,800013dc <uvmcopy+0x22>
    800013f8:	00a75793          	srl	a5,a4,0xa
    800013fc:	00c79b93          	sll	s7,a5,0xc
    80001400:	3ff77913          	and	s2,a4,1023
    80001404:	ee2ff0ef          	jal	80000ae6 <kalloc>
    80001408:	89aa                	mv	s3,a0
    8000140a:	c105                	beqz	a0,8000142a <uvmcopy+0x70>
    8000140c:	6605                	lui	a2,0x1
    8000140e:	85de                	mv	a1,s7
    80001410:	8e7ff0ef          	jal	80000cf6 <memmove>
    80001414:	874a                	mv	a4,s2
    80001416:	86ce                	mv	a3,s3
    80001418:	6605                	lui	a2,0x1
    8000141a:	85a6                	mv	a1,s1
    8000141c:	855a                	mv	a0,s6
    8000141e:	bedff0ef          	jal	8000100a <mappages>
    80001422:	dd4d                	beqz	a0,800013dc <uvmcopy+0x22>
    80001424:	854e                	mv	a0,s3
    80001426:	ddeff0ef          	jal	80000a04 <kfree>
    8000142a:	4685                	li	a3,1
    8000142c:	00c4d613          	srl	a2,s1,0xc
    80001430:	4581                	li	a1,0
    80001432:	855a                	mv	a0,s6
    80001434:	da3ff0ef          	jal	800011d6 <uvmunmap>
    80001438:	557d                	li	a0,-1
    8000143a:	a011                	j	8000143e <uvmcopy+0x84>
    8000143c:	4501                	li	a0,0
    8000143e:	60a6                	ld	ra,72(sp)
    80001440:	6406                	ld	s0,64(sp)
    80001442:	74e2                	ld	s1,56(sp)
    80001444:	7942                	ld	s2,48(sp)
    80001446:	79a2                	ld	s3,40(sp)
    80001448:	7a02                	ld	s4,32(sp)
    8000144a:	6ae2                	ld	s5,24(sp)
    8000144c:	6b42                	ld	s6,16(sp)
    8000144e:	6ba2                	ld	s7,8(sp)
    80001450:	6161                	add	sp,sp,80
    80001452:	8082                	ret
    80001454:	4501                	li	a0,0
    80001456:	8082                	ret

0000000080001458 <uvmclear>:
    80001458:	1141                	add	sp,sp,-16
    8000145a:	e406                	sd	ra,8(sp)
    8000145c:	e022                	sd	s0,0(sp)
    8000145e:	0800                	add	s0,sp,16
    80001460:	4601                	li	a2,0
    80001462:	ad1ff0ef          	jal	80000f32 <walk>
    80001466:	c901                	beqz	a0,80001476 <uvmclear+0x1e>
    80001468:	611c                	ld	a5,0(a0)
    8000146a:	9bbd                	and	a5,a5,-17
    8000146c:	e11c                	sd	a5,0(a0)
    8000146e:	60a2                	ld	ra,8(sp)
    80001470:	6402                	ld	s0,0(sp)
    80001472:	0141                	add	sp,sp,16
    80001474:	8082                	ret
    80001476:	00006517          	auipc	a0,0x6
    8000147a:	cea50513          	add	a0,a0,-790 # 80007160 <digits+0x148>
    8000147e:	b44ff0ef          	jal	800007c2 <panic>

0000000080001482 <copyinstr>:
    80001482:	cadd                	beqz	a3,80001538 <copyinstr+0xb6>
    80001484:	715d                	add	sp,sp,-80
    80001486:	e486                	sd	ra,72(sp)
    80001488:	e0a2                	sd	s0,64(sp)
    8000148a:	fc26                	sd	s1,56(sp)
    8000148c:	f84a                	sd	s2,48(sp)
    8000148e:	f44e                	sd	s3,40(sp)
    80001490:	f052                	sd	s4,32(sp)
    80001492:	ec56                	sd	s5,24(sp)
    80001494:	e85a                	sd	s6,16(sp)
    80001496:	e45e                	sd	s7,8(sp)
    80001498:	e062                	sd	s8,0(sp)
    8000149a:	0880                	add	s0,sp,80
    8000149c:	8aaa                	mv	s5,a0
    8000149e:	84ae                	mv	s1,a1
    800014a0:	8c32                	mv	s8,a2
    800014a2:	8bb6                	mv	s7,a3
    800014a4:	7a7d                	lui	s4,0xfffff
    800014a6:	6985                	lui	s3,0x1
    800014a8:	4b05                	li	s6,1
    800014aa:	a801                	j	800014ba <copyinstr+0x38>
    800014ac:	87a6                	mv	a5,s1
    800014ae:	a8b1                	j	8000150a <copyinstr+0x88>
    800014b0:	84b2                	mv	s1,a2
    800014b2:	01390c33          	add	s8,s2,s3
    800014b6:	060b8d63          	beqz	s7,80001530 <copyinstr+0xae>
    800014ba:	014c7933          	and	s2,s8,s4
    800014be:	85ca                	mv	a1,s2
    800014c0:	8556                	mv	a0,s5
    800014c2:	b0bff0ef          	jal	80000fcc <walkaddr>
    800014c6:	c53d                	beqz	a0,80001534 <copyinstr+0xb2>
    800014c8:	41890633          	sub	a2,s2,s8
    800014cc:	964e                	add	a2,a2,s3
    800014ce:	00cbf363          	bgeu	s7,a2,800014d4 <copyinstr+0x52>
    800014d2:	865e                	mv	a2,s7
    800014d4:	9562                	add	a0,a0,s8
    800014d6:	41250533          	sub	a0,a0,s2
    800014da:	de61                	beqz	a2,800014b2 <copyinstr+0x30>
    800014dc:	00054703          	lbu	a4,0(a0)
    800014e0:	d771                	beqz	a4,800014ac <copyinstr+0x2a>
    800014e2:	9626                	add	a2,a2,s1
    800014e4:	87a6                	mv	a5,s1
    800014e6:	1bfd                	add	s7,s7,-1 # fff <_entry-0x7ffff001>
    800014e8:	009b86b3          	add	a3,s7,s1
    800014ec:	409b04b3          	sub	s1,s6,s1
    800014f0:	94aa                	add	s1,s1,a0
    800014f2:	00e78023          	sb	a4,0(a5) # 1000 <_entry-0x7ffff000>
    800014f6:	40f68bb3          	sub	s7,a3,a5
    800014fa:	00f48733          	add	a4,s1,a5
    800014fe:	0785                	add	a5,a5,1
    80001500:	faf608e3          	beq	a2,a5,800014b0 <copyinstr+0x2e>
    80001504:	00074703          	lbu	a4,0(a4) # fffffffffffff000 <end+0xffffffff7ffde498>
    80001508:	f76d                	bnez	a4,800014f2 <copyinstr+0x70>
    8000150a:	00078023          	sb	zero,0(a5)
    8000150e:	4785                	li	a5,1
    80001510:	0017b793          	seqz	a5,a5
    80001514:	40f00533          	neg	a0,a5
    80001518:	60a6                	ld	ra,72(sp)
    8000151a:	6406                	ld	s0,64(sp)
    8000151c:	74e2                	ld	s1,56(sp)
    8000151e:	7942                	ld	s2,48(sp)
    80001520:	79a2                	ld	s3,40(sp)
    80001522:	7a02                	ld	s4,32(sp)
    80001524:	6ae2                	ld	s5,24(sp)
    80001526:	6b42                	ld	s6,16(sp)
    80001528:	6ba2                	ld	s7,8(sp)
    8000152a:	6c02                	ld	s8,0(sp)
    8000152c:	6161                	add	sp,sp,80
    8000152e:	8082                	ret
    80001530:	4781                	li	a5,0
    80001532:	bff9                	j	80001510 <copyinstr+0x8e>
    80001534:	557d                	li	a0,-1
    80001536:	b7cd                	j	80001518 <copyinstr+0x96>
    80001538:	4781                	li	a5,0
    8000153a:	0017b793          	seqz	a5,a5
    8000153e:	40f00533          	neg	a0,a5
    80001542:	8082                	ret

0000000080001544 <ismapped>:
    80001544:	1141                	add	sp,sp,-16
    80001546:	e406                	sd	ra,8(sp)
    80001548:	e022                	sd	s0,0(sp)
    8000154a:	0800                	add	s0,sp,16
    8000154c:	4601                	li	a2,0
    8000154e:	9e5ff0ef          	jal	80000f32 <walk>
    80001552:	c519                	beqz	a0,80001560 <ismapped+0x1c>
    80001554:	6108                	ld	a0,0(a0)
    80001556:	8905                	and	a0,a0,1
    80001558:	60a2                	ld	ra,8(sp)
    8000155a:	6402                	ld	s0,0(sp)
    8000155c:	0141                	add	sp,sp,16
    8000155e:	8082                	ret
    80001560:	4501                	li	a0,0
    80001562:	bfdd                	j	80001558 <ismapped+0x14>

0000000080001564 <vmfault>:
    80001564:	7179                	add	sp,sp,-48
    80001566:	f406                	sd	ra,40(sp)
    80001568:	f022                	sd	s0,32(sp)
    8000156a:	ec26                	sd	s1,24(sp)
    8000156c:	e84a                	sd	s2,16(sp)
    8000156e:	e44e                	sd	s3,8(sp)
    80001570:	e052                	sd	s4,0(sp)
    80001572:	1800                	add	s0,sp,48
    80001574:	892a                	mv	s2,a0
    80001576:	84ae                	mv	s1,a1
    80001578:	314000ef          	jal	8000188c <myproc>
    8000157c:	653c                	ld	a5,72(a0)
    8000157e:	00f4ec63          	bltu	s1,a5,80001596 <vmfault+0x32>
    80001582:	4901                	li	s2,0
    80001584:	854a                	mv	a0,s2
    80001586:	70a2                	ld	ra,40(sp)
    80001588:	7402                	ld	s0,32(sp)
    8000158a:	64e2                	ld	s1,24(sp)
    8000158c:	6942                	ld	s2,16(sp)
    8000158e:	69a2                	ld	s3,8(sp)
    80001590:	6a02                	ld	s4,0(sp)
    80001592:	6145                	add	sp,sp,48
    80001594:	8082                	ret
    80001596:	89aa                	mv	s3,a0
    80001598:	77fd                	lui	a5,0xfffff
    8000159a:	8cfd                	and	s1,s1,a5
    8000159c:	85a6                	mv	a1,s1
    8000159e:	854a                	mv	a0,s2
    800015a0:	fa5ff0ef          	jal	80001544 <ismapped>
    800015a4:	4901                	li	s2,0
    800015a6:	fd79                	bnez	a0,80001584 <vmfault+0x20>
    800015a8:	d3eff0ef          	jal	80000ae6 <kalloc>
    800015ac:	8a2a                	mv	s4,a0
    800015ae:	d979                	beqz	a0,80001584 <vmfault+0x20>
    800015b0:	892a                	mv	s2,a0
    800015b2:	6605                	lui	a2,0x1
    800015b4:	4581                	li	a1,0
    800015b6:	ed4ff0ef          	jal	80000c8a <memset>
    800015ba:	4759                	li	a4,22
    800015bc:	86d2                	mv	a3,s4
    800015be:	6605                	lui	a2,0x1
    800015c0:	85a6                	mv	a1,s1
    800015c2:	0509b503          	ld	a0,80(s3) # 1050 <_entry-0x7fffefb0>
    800015c6:	a45ff0ef          	jal	8000100a <mappages>
    800015ca:	dd4d                	beqz	a0,80001584 <vmfault+0x20>
    800015cc:	8552                	mv	a0,s4
    800015ce:	c36ff0ef          	jal	80000a04 <kfree>
    800015d2:	4901                	li	s2,0
    800015d4:	bf45                	j	80001584 <vmfault+0x20>

00000000800015d6 <copyout>:
    800015d6:	cec1                	beqz	a3,8000166e <copyout+0x98>
    800015d8:	711d                	add	sp,sp,-96
    800015da:	ec86                	sd	ra,88(sp)
    800015dc:	e8a2                	sd	s0,80(sp)
    800015de:	e4a6                	sd	s1,72(sp)
    800015e0:	e0ca                	sd	s2,64(sp)
    800015e2:	fc4e                	sd	s3,56(sp)
    800015e4:	f852                	sd	s4,48(sp)
    800015e6:	f456                	sd	s5,40(sp)
    800015e8:	f05a                	sd	s6,32(sp)
    800015ea:	ec5e                	sd	s7,24(sp)
    800015ec:	e862                	sd	s8,16(sp)
    800015ee:	e466                	sd	s9,8(sp)
    800015f0:	e06a                	sd	s10,0(sp)
    800015f2:	1080                	add	s0,sp,96
    800015f4:	8baa                	mv	s7,a0
    800015f6:	8a2e                	mv	s4,a1
    800015f8:	8b32                	mv	s6,a2
    800015fa:	89b6                	mv	s3,a3
    800015fc:	74fd                	lui	s1,0xfffff
    800015fe:	8ced                	and	s1,s1,a1
    80001600:	57fd                	li	a5,-1
    80001602:	83e9                	srl	a5,a5,0x1a
    80001604:	0697e763          	bltu	a5,s1,80001672 <copyout+0x9c>
    80001608:	6c85                	lui	s9,0x1
    8000160a:	8c3e                	mv	s8,a5
    8000160c:	a015                	j	80001630 <copyout+0x5a>
    8000160e:	409a0533          	sub	a0,s4,s1
    80001612:	0009061b          	sext.w	a2,s2
    80001616:	85da                	mv	a1,s6
    80001618:	9556                	add	a0,a0,s5
    8000161a:	edcff0ef          	jal	80000cf6 <memmove>
    8000161e:	412989b3          	sub	s3,s3,s2
    80001622:	9b4a                	add	s6,s6,s2
    80001624:	04098363          	beqz	s3,8000166a <copyout+0x94>
    80001628:	8a6a                	mv	s4,s10
    8000162a:	84ea                	mv	s1,s10
    8000162c:	05ac6563          	bltu	s8,s10,80001676 <copyout+0xa0>
    80001630:	85a6                	mv	a1,s1
    80001632:	855e                	mv	a0,s7
    80001634:	999ff0ef          	jal	80000fcc <walkaddr>
    80001638:	8aaa                	mv	s5,a0
    8000163a:	e901                	bnez	a0,8000164a <copyout+0x74>
    8000163c:	4601                	li	a2,0
    8000163e:	85a6                	mv	a1,s1
    80001640:	855e                	mv	a0,s7
    80001642:	f23ff0ef          	jal	80001564 <vmfault>
    80001646:	8aaa                	mv	s5,a0
    80001648:	c90d                	beqz	a0,8000167a <copyout+0xa4>
    8000164a:	4601                	li	a2,0
    8000164c:	85a6                	mv	a1,s1
    8000164e:	855e                	mv	a0,s7
    80001650:	8e3ff0ef          	jal	80000f32 <walk>
    80001654:	611c                	ld	a5,0(a0)
    80001656:	8b91                	and	a5,a5,4
    80001658:	c39d                	beqz	a5,8000167e <copyout+0xa8>
    8000165a:	01948d33          	add	s10,s1,s9
    8000165e:	414d0933          	sub	s2,s10,s4
    80001662:	fb29f6e3          	bgeu	s3,s2,8000160e <copyout+0x38>
    80001666:	894e                	mv	s2,s3
    80001668:	b75d                	j	8000160e <copyout+0x38>
    8000166a:	4501                	li	a0,0
    8000166c:	a811                	j	80001680 <copyout+0xaa>
    8000166e:	4501                	li	a0,0
    80001670:	8082                	ret
    80001672:	557d                	li	a0,-1
    80001674:	a031                	j	80001680 <copyout+0xaa>
    80001676:	557d                	li	a0,-1
    80001678:	a021                	j	80001680 <copyout+0xaa>
    8000167a:	557d                	li	a0,-1
    8000167c:	a011                	j	80001680 <copyout+0xaa>
    8000167e:	557d                	li	a0,-1
    80001680:	60e6                	ld	ra,88(sp)
    80001682:	6446                	ld	s0,80(sp)
    80001684:	64a6                	ld	s1,72(sp)
    80001686:	6906                	ld	s2,64(sp)
    80001688:	79e2                	ld	s3,56(sp)
    8000168a:	7a42                	ld	s4,48(sp)
    8000168c:	7aa2                	ld	s5,40(sp)
    8000168e:	7b02                	ld	s6,32(sp)
    80001690:	6be2                	ld	s7,24(sp)
    80001692:	6c42                	ld	s8,16(sp)
    80001694:	6ca2                	ld	s9,8(sp)
    80001696:	6d02                	ld	s10,0(sp)
    80001698:	6125                	add	sp,sp,96
    8000169a:	8082                	ret

000000008000169c <copyin>:
    8000169c:	c6d9                	beqz	a3,8000172a <copyin+0x8e>
    8000169e:	715d                	add	sp,sp,-80
    800016a0:	e486                	sd	ra,72(sp)
    800016a2:	e0a2                	sd	s0,64(sp)
    800016a4:	fc26                	sd	s1,56(sp)
    800016a6:	f84a                	sd	s2,48(sp)
    800016a8:	f44e                	sd	s3,40(sp)
    800016aa:	f052                	sd	s4,32(sp)
    800016ac:	ec56                	sd	s5,24(sp)
    800016ae:	e85a                	sd	s6,16(sp)
    800016b0:	e45e                	sd	s7,8(sp)
    800016b2:	e062                	sd	s8,0(sp)
    800016b4:	0880                	add	s0,sp,80
    800016b6:	8baa                	mv	s7,a0
    800016b8:	8aae                	mv	s5,a1
    800016ba:	8932                	mv	s2,a2
    800016bc:	8a36                	mv	s4,a3
    800016be:	7c7d                	lui	s8,0xfffff
    800016c0:	6b05                	lui	s6,0x1
    800016c2:	a80d                	j	800016f4 <copyin+0x58>
    800016c4:	4601                	li	a2,0
    800016c6:	85ce                	mv	a1,s3
    800016c8:	855e                	mv	a0,s7
    800016ca:	e9bff0ef          	jal	80001564 <vmfault>
    800016ce:	e915                	bnez	a0,80001702 <copyin+0x66>
    800016d0:	557d                	li	a0,-1
    800016d2:	a081                	j	80001712 <copyin+0x76>
    800016d4:	41390933          	sub	s2,s2,s3
    800016d8:	0004861b          	sext.w	a2,s1
    800016dc:	00a905b3          	add	a1,s2,a0
    800016e0:	8556                	mv	a0,s5
    800016e2:	e14ff0ef          	jal	80000cf6 <memmove>
    800016e6:	409a0a33          	sub	s4,s4,s1
    800016ea:	9aa6                	add	s5,s5,s1
    800016ec:	01698933          	add	s2,s3,s6
    800016f0:	020a0063          	beqz	s4,80001710 <copyin+0x74>
    800016f4:	018979b3          	and	s3,s2,s8
    800016f8:	85ce                	mv	a1,s3
    800016fa:	855e                	mv	a0,s7
    800016fc:	8d1ff0ef          	jal	80000fcc <walkaddr>
    80001700:	d171                	beqz	a0,800016c4 <copyin+0x28>
    80001702:	412984b3          	sub	s1,s3,s2
    80001706:	94da                	add	s1,s1,s6
    80001708:	fc9a76e3          	bgeu	s4,s1,800016d4 <copyin+0x38>
    8000170c:	84d2                	mv	s1,s4
    8000170e:	b7d9                	j	800016d4 <copyin+0x38>
    80001710:	4501                	li	a0,0
    80001712:	60a6                	ld	ra,72(sp)
    80001714:	6406                	ld	s0,64(sp)
    80001716:	74e2                	ld	s1,56(sp)
    80001718:	7942                	ld	s2,48(sp)
    8000171a:	79a2                	ld	s3,40(sp)
    8000171c:	7a02                	ld	s4,32(sp)
    8000171e:	6ae2                	ld	s5,24(sp)
    80001720:	6b42                	ld	s6,16(sp)
    80001722:	6ba2                	ld	s7,8(sp)
    80001724:	6c02                	ld	s8,0(sp)
    80001726:	6161                	add	sp,sp,80
    80001728:	8082                	ret
    8000172a:	4501                	li	a0,0
    8000172c:	8082                	ret

000000008000172e <proc_mapstacks>:
    8000172e:	7139                	add	sp,sp,-64
    80001730:	fc06                	sd	ra,56(sp)
    80001732:	f822                	sd	s0,48(sp)
    80001734:	f426                	sd	s1,40(sp)
    80001736:	f04a                	sd	s2,32(sp)
    80001738:	ec4e                	sd	s3,24(sp)
    8000173a:	e852                	sd	s4,16(sp)
    8000173c:	e456                	sd	s5,8(sp)
    8000173e:	e05a                	sd	s6,0(sp)
    80001740:	0080                	add	s0,sp,64
    80001742:	8b2a                	mv	s6,a0
    80001744:	0000e497          	auipc	s1,0xe
    80001748:	64448493          	add	s1,s1,1604 # 8000fd88 <proc>
    8000174c:	8aa6                	mv	s5,s1
    8000174e:	00006a17          	auipc	s4,0x6
    80001752:	8b2a0a13          	add	s4,s4,-1870 # 80007000 <etext>
    80001756:	04000937          	lui	s2,0x4000
    8000175a:	197d                	add	s2,s2,-1 # 3ffffff <_entry-0x7c000001>
    8000175c:	0932                	sll	s2,s2,0xc
    8000175e:	00014997          	auipc	s3,0x14
    80001762:	02a98993          	add	s3,s3,42 # 80015788 <tickslock>
    80001766:	b80ff0ef          	jal	80000ae6 <kalloc>
    8000176a:	862a                	mv	a2,a0
    8000176c:	c121                	beqz	a0,800017ac <proc_mapstacks+0x7e>
    8000176e:	415485b3          	sub	a1,s1,s5
    80001772:	858d                	sra	a1,a1,0x3
    80001774:	000a3783          	ld	a5,0(s4)
    80001778:	02f585b3          	mul	a1,a1,a5
    8000177c:	2585                	addw	a1,a1,1
    8000177e:	00d5959b          	sllw	a1,a1,0xd
    80001782:	4719                	li	a4,6
    80001784:	6685                	lui	a3,0x1
    80001786:	40b905b3          	sub	a1,s2,a1
    8000178a:	855a                	mv	a0,s6
    8000178c:	92fff0ef          	jal	800010ba <kvmmap>
    80001790:	16848493          	add	s1,s1,360
    80001794:	fd3499e3          	bne	s1,s3,80001766 <proc_mapstacks+0x38>
    80001798:	70e2                	ld	ra,56(sp)
    8000179a:	7442                	ld	s0,48(sp)
    8000179c:	74a2                	ld	s1,40(sp)
    8000179e:	7902                	ld	s2,32(sp)
    800017a0:	69e2                	ld	s3,24(sp)
    800017a2:	6a42                	ld	s4,16(sp)
    800017a4:	6aa2                	ld	s5,8(sp)
    800017a6:	6b02                	ld	s6,0(sp)
    800017a8:	6121                	add	sp,sp,64
    800017aa:	8082                	ret
    800017ac:	00006517          	auipc	a0,0x6
    800017b0:	9f450513          	add	a0,a0,-1548 # 800071a0 <states.1753+0x30>
    800017b4:	80eff0ef          	jal	800007c2 <panic>

00000000800017b8 <procinit>:
    800017b8:	7139                	add	sp,sp,-64
    800017ba:	fc06                	sd	ra,56(sp)
    800017bc:	f822                	sd	s0,48(sp)
    800017be:	f426                	sd	s1,40(sp)
    800017c0:	f04a                	sd	s2,32(sp)
    800017c2:	ec4e                	sd	s3,24(sp)
    800017c4:	e852                	sd	s4,16(sp)
    800017c6:	e456                	sd	s5,8(sp)
    800017c8:	e05a                	sd	s6,0(sp)
    800017ca:	0080                	add	s0,sp,64
    800017cc:	00006597          	auipc	a1,0x6
    800017d0:	9dc58593          	add	a1,a1,-1572 # 800071a8 <states.1753+0x38>
    800017d4:	0000e517          	auipc	a0,0xe
    800017d8:	18450513          	add	a0,a0,388 # 8000f958 <pid_lock>
    800017dc:	b5aff0ef          	jal	80000b36 <initlock>
    800017e0:	00006597          	auipc	a1,0x6
    800017e4:	9d058593          	add	a1,a1,-1584 # 800071b0 <states.1753+0x40>
    800017e8:	0000e517          	auipc	a0,0xe
    800017ec:	18850513          	add	a0,a0,392 # 8000f970 <wait_lock>
    800017f0:	b46ff0ef          	jal	80000b36 <initlock>
    800017f4:	0000e497          	auipc	s1,0xe
    800017f8:	59448493          	add	s1,s1,1428 # 8000fd88 <proc>
    800017fc:	00006b17          	auipc	s6,0x6
    80001800:	9c4b0b13          	add	s6,s6,-1596 # 800071c0 <states.1753+0x50>
    80001804:	8aa6                	mv	s5,s1
    80001806:	00005a17          	auipc	s4,0x5
    8000180a:	7faa0a13          	add	s4,s4,2042 # 80007000 <etext>
    8000180e:	04000937          	lui	s2,0x4000
    80001812:	197d                	add	s2,s2,-1 # 3ffffff <_entry-0x7c000001>
    80001814:	0932                	sll	s2,s2,0xc
    80001816:	00014997          	auipc	s3,0x14
    8000181a:	f7298993          	add	s3,s3,-142 # 80015788 <tickslock>
    8000181e:	85da                	mv	a1,s6
    80001820:	8526                	mv	a0,s1
    80001822:	b14ff0ef          	jal	80000b36 <initlock>
    80001826:	0004ac23          	sw	zero,24(s1)
    8000182a:	415487b3          	sub	a5,s1,s5
    8000182e:	878d                	sra	a5,a5,0x3
    80001830:	000a3703          	ld	a4,0(s4)
    80001834:	02e787b3          	mul	a5,a5,a4
    80001838:	2785                	addw	a5,a5,1 # fffffffffffff001 <end+0xffffffff7ffde499>
    8000183a:	00d7979b          	sllw	a5,a5,0xd
    8000183e:	40f907b3          	sub	a5,s2,a5
    80001842:	e0bc                	sd	a5,64(s1)
    80001844:	16848493          	add	s1,s1,360
    80001848:	fd349be3          	bne	s1,s3,8000181e <procinit+0x66>
    8000184c:	70e2                	ld	ra,56(sp)
    8000184e:	7442                	ld	s0,48(sp)
    80001850:	74a2                	ld	s1,40(sp)
    80001852:	7902                	ld	s2,32(sp)
    80001854:	69e2                	ld	s3,24(sp)
    80001856:	6a42                	ld	s4,16(sp)
    80001858:	6aa2                	ld	s5,8(sp)
    8000185a:	6b02                	ld	s6,0(sp)
    8000185c:	6121                	add	sp,sp,64
    8000185e:	8082                	ret

0000000080001860 <cpuid>:
    80001860:	1141                	add	sp,sp,-16
    80001862:	e422                	sd	s0,8(sp)
    80001864:	0800                	add	s0,sp,16
    80001866:	8512                	mv	a0,tp
    80001868:	2501                	sext.w	a0,a0
    8000186a:	6422                	ld	s0,8(sp)
    8000186c:	0141                	add	sp,sp,16
    8000186e:	8082                	ret

0000000080001870 <mycpu>:
    80001870:	1141                	add	sp,sp,-16
    80001872:	e422                	sd	s0,8(sp)
    80001874:	0800                	add	s0,sp,16
    80001876:	8792                	mv	a5,tp
    80001878:	2781                	sext.w	a5,a5
    8000187a:	079e                	sll	a5,a5,0x7
    8000187c:	0000e517          	auipc	a0,0xe
    80001880:	10c50513          	add	a0,a0,268 # 8000f988 <cpus>
    80001884:	953e                	add	a0,a0,a5
    80001886:	6422                	ld	s0,8(sp)
    80001888:	0141                	add	sp,sp,16
    8000188a:	8082                	ret

000000008000188c <myproc>:
    8000188c:	1101                	add	sp,sp,-32
    8000188e:	ec06                	sd	ra,24(sp)
    80001890:	e822                	sd	s0,16(sp)
    80001892:	e426                	sd	s1,8(sp)
    80001894:	1000                	add	s0,sp,32
    80001896:	ae0ff0ef          	jal	80000b76 <push_off>
    8000189a:	8792                	mv	a5,tp
    8000189c:	2781                	sext.w	a5,a5
    8000189e:	079e                	sll	a5,a5,0x7
    800018a0:	0000e717          	auipc	a4,0xe
    800018a4:	0b870713          	add	a4,a4,184 # 8000f958 <pid_lock>
    800018a8:	97ba                	add	a5,a5,a4
    800018aa:	7b84                	ld	s1,48(a5)
    800018ac:	b4eff0ef          	jal	80000bfa <pop_off>
    800018b0:	8526                	mv	a0,s1
    800018b2:	60e2                	ld	ra,24(sp)
    800018b4:	6442                	ld	s0,16(sp)
    800018b6:	64a2                	ld	s1,8(sp)
    800018b8:	6105                	add	sp,sp,32
    800018ba:	8082                	ret

00000000800018bc <forkret>:
    800018bc:	7179                	add	sp,sp,-48
    800018be:	f406                	sd	ra,40(sp)
    800018c0:	f022                	sd	s0,32(sp)
    800018c2:	ec26                	sd	s1,24(sp)
    800018c4:	1800                	add	s0,sp,48
    800018c6:	fc7ff0ef          	jal	8000188c <myproc>
    800018ca:	84aa                	mv	s1,a0
    800018cc:	b82ff0ef          	jal	80000c4e <release>
    800018d0:	00006797          	auipc	a5,0x6
    800018d4:	f5078793          	add	a5,a5,-176 # 80007820 <first.1703>
    800018d8:	439c                	lw	a5,0(a5)
    800018da:	cf8d                	beqz	a5,80001914 <forkret+0x58>
    800018dc:	4505                	li	a0,1
    800018de:	3d1010ef          	jal	800034ae <fsinit>
    800018e2:	00006797          	auipc	a5,0x6
    800018e6:	f207af23          	sw	zero,-194(a5) # 80007820 <first.1703>
    800018ea:	0ff0000f          	fence
    800018ee:	00006517          	auipc	a0,0x6
    800018f2:	8da50513          	add	a0,a0,-1830 # 800071c8 <states.1753+0x58>
    800018f6:	fca43823          	sd	a0,-48(s0)
    800018fa:	fc043c23          	sd	zero,-40(s0)
    800018fe:	fd040593          	add	a1,s0,-48
    80001902:	4b3020ef          	jal	800045b4 <kexec>
    80001906:	6cbc                	ld	a5,88(s1)
    80001908:	fba8                	sd	a0,112(a5)
    8000190a:	6cbc                	ld	a5,88(s1)
    8000190c:	7bb8                	ld	a4,112(a5)
    8000190e:	57fd                	li	a5,-1
    80001910:	02f70d63          	beq	a4,a5,8000194a <forkret+0x8e>
    80001914:	2bd000ef          	jal	800023d0 <prepare_return>
    80001918:	68a8                	ld	a0,80(s1)
    8000191a:	8131                	srl	a0,a0,0xc
    8000191c:	04000737          	lui	a4,0x4000
    80001920:	00004797          	auipc	a5,0x4
    80001924:	78078793          	add	a5,a5,1920 # 800060a0 <userret>
    80001928:	00004697          	auipc	a3,0x4
    8000192c:	6d868693          	add	a3,a3,1752 # 80006000 <_trampoline>
    80001930:	8f95                	sub	a5,a5,a3
    80001932:	177d                	add	a4,a4,-1 # 3ffffff <_entry-0x7c000001>
    80001934:	0732                	sll	a4,a4,0xc
    80001936:	97ba                	add	a5,a5,a4
    80001938:	577d                	li	a4,-1
    8000193a:	177e                	sll	a4,a4,0x3f
    8000193c:	8d59                	or	a0,a0,a4
    8000193e:	9782                	jalr	a5
    80001940:	70a2                	ld	ra,40(sp)
    80001942:	7402                	ld	s0,32(sp)
    80001944:	64e2                	ld	s1,24(sp)
    80001946:	6145                	add	sp,sp,48
    80001948:	8082                	ret
    8000194a:	00006517          	auipc	a0,0x6
    8000194e:	88650513          	add	a0,a0,-1914 # 800071d0 <states.1753+0x60>
    80001952:	e71fe0ef          	jal	800007c2 <panic>

0000000080001956 <allocpid>:
    80001956:	1101                	add	sp,sp,-32
    80001958:	ec06                	sd	ra,24(sp)
    8000195a:	e822                	sd	s0,16(sp)
    8000195c:	e426                	sd	s1,8(sp)
    8000195e:	e04a                	sd	s2,0(sp)
    80001960:	1000                	add	s0,sp,32
    80001962:	0000e917          	auipc	s2,0xe
    80001966:	ff690913          	add	s2,s2,-10 # 8000f958 <pid_lock>
    8000196a:	854a                	mv	a0,s2
    8000196c:	a4aff0ef          	jal	80000bb6 <acquire>
    80001970:	00006797          	auipc	a5,0x6
    80001974:	eb478793          	add	a5,a5,-332 # 80007824 <nextpid>
    80001978:	4384                	lw	s1,0(a5)
    8000197a:	0014871b          	addw	a4,s1,1
    8000197e:	c398                	sw	a4,0(a5)
    80001980:	854a                	mv	a0,s2
    80001982:	accff0ef          	jal	80000c4e <release>
    80001986:	8526                	mv	a0,s1
    80001988:	60e2                	ld	ra,24(sp)
    8000198a:	6442                	ld	s0,16(sp)
    8000198c:	64a2                	ld	s1,8(sp)
    8000198e:	6902                	ld	s2,0(sp)
    80001990:	6105                	add	sp,sp,32
    80001992:	8082                	ret

0000000080001994 <proc_pagetable>:
    80001994:	1101                	add	sp,sp,-32
    80001996:	ec06                	sd	ra,24(sp)
    80001998:	e822                	sd	s0,16(sp)
    8000199a:	e426                	sd	s1,8(sp)
    8000199c:	e04a                	sd	s2,0(sp)
    8000199e:	1000                	add	s0,sp,32
    800019a0:	892a                	mv	s2,a0
    800019a2:	80fff0ef          	jal	800011b0 <uvmcreate>
    800019a6:	84aa                	mv	s1,a0
    800019a8:	cd05                	beqz	a0,800019e0 <proc_pagetable+0x4c>
    800019aa:	4729                	li	a4,10
    800019ac:	00004697          	auipc	a3,0x4
    800019b0:	65468693          	add	a3,a3,1620 # 80006000 <_trampoline>
    800019b4:	6605                	lui	a2,0x1
    800019b6:	040005b7          	lui	a1,0x4000
    800019ba:	15fd                	add	a1,a1,-1 # 3ffffff <_entry-0x7c000001>
    800019bc:	05b2                	sll	a1,a1,0xc
    800019be:	e4cff0ef          	jal	8000100a <mappages>
    800019c2:	02054663          	bltz	a0,800019ee <proc_pagetable+0x5a>
    800019c6:	4719                	li	a4,6
    800019c8:	05893683          	ld	a3,88(s2)
    800019cc:	6605                	lui	a2,0x1
    800019ce:	020005b7          	lui	a1,0x2000
    800019d2:	15fd                	add	a1,a1,-1 # 1ffffff <_entry-0x7e000001>
    800019d4:	05b6                	sll	a1,a1,0xd
    800019d6:	8526                	mv	a0,s1
    800019d8:	e32ff0ef          	jal	8000100a <mappages>
    800019dc:	00054f63          	bltz	a0,800019fa <proc_pagetable+0x66>
    800019e0:	8526                	mv	a0,s1
    800019e2:	60e2                	ld	ra,24(sp)
    800019e4:	6442                	ld	s0,16(sp)
    800019e6:	64a2                	ld	s1,8(sp)
    800019e8:	6902                	ld	s2,0(sp)
    800019ea:	6105                	add	sp,sp,32
    800019ec:	8082                	ret
    800019ee:	4581                	li	a1,0
    800019f0:	8526                	mv	a0,s1
    800019f2:	999ff0ef          	jal	8000138a <uvmfree>
    800019f6:	4481                	li	s1,0
    800019f8:	b7e5                	j	800019e0 <proc_pagetable+0x4c>
    800019fa:	4681                	li	a3,0
    800019fc:	4605                	li	a2,1
    800019fe:	040005b7          	lui	a1,0x4000
    80001a02:	15fd                	add	a1,a1,-1 # 3ffffff <_entry-0x7c000001>
    80001a04:	05b2                	sll	a1,a1,0xc
    80001a06:	8526                	mv	a0,s1
    80001a08:	fceff0ef          	jal	800011d6 <uvmunmap>
    80001a0c:	4581                	li	a1,0
    80001a0e:	8526                	mv	a0,s1
    80001a10:	97bff0ef          	jal	8000138a <uvmfree>
    80001a14:	4481                	li	s1,0
    80001a16:	b7e9                	j	800019e0 <proc_pagetable+0x4c>

0000000080001a18 <proc_freepagetable>:
    80001a18:	1101                	add	sp,sp,-32
    80001a1a:	ec06                	sd	ra,24(sp)
    80001a1c:	e822                	sd	s0,16(sp)
    80001a1e:	e426                	sd	s1,8(sp)
    80001a20:	e04a                	sd	s2,0(sp)
    80001a22:	1000                	add	s0,sp,32
    80001a24:	84aa                	mv	s1,a0
    80001a26:	892e                	mv	s2,a1
    80001a28:	4681                	li	a3,0
    80001a2a:	4605                	li	a2,1
    80001a2c:	040005b7          	lui	a1,0x4000
    80001a30:	15fd                	add	a1,a1,-1 # 3ffffff <_entry-0x7c000001>
    80001a32:	05b2                	sll	a1,a1,0xc
    80001a34:	fa2ff0ef          	jal	800011d6 <uvmunmap>
    80001a38:	4681                	li	a3,0
    80001a3a:	4605                	li	a2,1
    80001a3c:	020005b7          	lui	a1,0x2000
    80001a40:	15fd                	add	a1,a1,-1 # 1ffffff <_entry-0x7e000001>
    80001a42:	05b6                	sll	a1,a1,0xd
    80001a44:	8526                	mv	a0,s1
    80001a46:	f90ff0ef          	jal	800011d6 <uvmunmap>
    80001a4a:	85ca                	mv	a1,s2
    80001a4c:	8526                	mv	a0,s1
    80001a4e:	93dff0ef          	jal	8000138a <uvmfree>
    80001a52:	60e2                	ld	ra,24(sp)
    80001a54:	6442                	ld	s0,16(sp)
    80001a56:	64a2                	ld	s1,8(sp)
    80001a58:	6902                	ld	s2,0(sp)
    80001a5a:	6105                	add	sp,sp,32
    80001a5c:	8082                	ret

0000000080001a5e <freeproc>:
    80001a5e:	1101                	add	sp,sp,-32
    80001a60:	ec06                	sd	ra,24(sp)
    80001a62:	e822                	sd	s0,16(sp)
    80001a64:	e426                	sd	s1,8(sp)
    80001a66:	1000                	add	s0,sp,32
    80001a68:	84aa                	mv	s1,a0
    80001a6a:	6d28                	ld	a0,88(a0)
    80001a6c:	c119                	beqz	a0,80001a72 <freeproc+0x14>
    80001a6e:	f97fe0ef          	jal	80000a04 <kfree>
    80001a72:	0404bc23          	sd	zero,88(s1)
    80001a76:	68a8                	ld	a0,80(s1)
    80001a78:	c501                	beqz	a0,80001a80 <freeproc+0x22>
    80001a7a:	64ac                	ld	a1,72(s1)
    80001a7c:	f9dff0ef          	jal	80001a18 <proc_freepagetable>
    80001a80:	0404b823          	sd	zero,80(s1)
    80001a84:	0404b423          	sd	zero,72(s1)
    80001a88:	0204a823          	sw	zero,48(s1)
    80001a8c:	0204bc23          	sd	zero,56(s1)
    80001a90:	14048c23          	sb	zero,344(s1)
    80001a94:	0204b023          	sd	zero,32(s1)
    80001a98:	0204a423          	sw	zero,40(s1)
    80001a9c:	0204a623          	sw	zero,44(s1)
    80001aa0:	0004ac23          	sw	zero,24(s1)
    80001aa4:	60e2                	ld	ra,24(sp)
    80001aa6:	6442                	ld	s0,16(sp)
    80001aa8:	64a2                	ld	s1,8(sp)
    80001aaa:	6105                	add	sp,sp,32
    80001aac:	8082                	ret

0000000080001aae <allocproc>:
    80001aae:	1101                	add	sp,sp,-32
    80001ab0:	ec06                	sd	ra,24(sp)
    80001ab2:	e822                	sd	s0,16(sp)
    80001ab4:	e426                	sd	s1,8(sp)
    80001ab6:	e04a                	sd	s2,0(sp)
    80001ab8:	1000                	add	s0,sp,32
    80001aba:	0000e497          	auipc	s1,0xe
    80001abe:	2ce48493          	add	s1,s1,718 # 8000fd88 <proc>
    80001ac2:	00014917          	auipc	s2,0x14
    80001ac6:	cc690913          	add	s2,s2,-826 # 80015788 <tickslock>
    80001aca:	8526                	mv	a0,s1
    80001acc:	8eaff0ef          	jal	80000bb6 <acquire>
    80001ad0:	4c9c                	lw	a5,24(s1)
    80001ad2:	cb91                	beqz	a5,80001ae6 <allocproc+0x38>
    80001ad4:	8526                	mv	a0,s1
    80001ad6:	978ff0ef          	jal	80000c4e <release>
    80001ada:	16848493          	add	s1,s1,360
    80001ade:	ff2496e3          	bne	s1,s2,80001aca <allocproc+0x1c>
    80001ae2:	4481                	li	s1,0
    80001ae4:	a089                	j	80001b26 <allocproc+0x78>
    80001ae6:	e71ff0ef          	jal	80001956 <allocpid>
    80001aea:	d888                	sw	a0,48(s1)
    80001aec:	4785                	li	a5,1
    80001aee:	cc9c                	sw	a5,24(s1)
    80001af0:	ff7fe0ef          	jal	80000ae6 <kalloc>
    80001af4:	892a                	mv	s2,a0
    80001af6:	eca8                	sd	a0,88(s1)
    80001af8:	cd15                	beqz	a0,80001b34 <allocproc+0x86>
    80001afa:	8526                	mv	a0,s1
    80001afc:	e99ff0ef          	jal	80001994 <proc_pagetable>
    80001b00:	892a                	mv	s2,a0
    80001b02:	e8a8                	sd	a0,80(s1)
    80001b04:	c121                	beqz	a0,80001b44 <allocproc+0x96>
    80001b06:	07000613          	li	a2,112
    80001b0a:	4581                	li	a1,0
    80001b0c:	06048513          	add	a0,s1,96
    80001b10:	97aff0ef          	jal	80000c8a <memset>
    80001b14:	00000797          	auipc	a5,0x0
    80001b18:	da878793          	add	a5,a5,-600 # 800018bc <forkret>
    80001b1c:	f0bc                	sd	a5,96(s1)
    80001b1e:	60bc                	ld	a5,64(s1)
    80001b20:	6705                	lui	a4,0x1
    80001b22:	97ba                	add	a5,a5,a4
    80001b24:	f4bc                	sd	a5,104(s1)
    80001b26:	8526                	mv	a0,s1
    80001b28:	60e2                	ld	ra,24(sp)
    80001b2a:	6442                	ld	s0,16(sp)
    80001b2c:	64a2                	ld	s1,8(sp)
    80001b2e:	6902                	ld	s2,0(sp)
    80001b30:	6105                	add	sp,sp,32
    80001b32:	8082                	ret
    80001b34:	8526                	mv	a0,s1
    80001b36:	f29ff0ef          	jal	80001a5e <freeproc>
    80001b3a:	8526                	mv	a0,s1
    80001b3c:	912ff0ef          	jal	80000c4e <release>
    80001b40:	84ca                	mv	s1,s2
    80001b42:	b7d5                	j	80001b26 <allocproc+0x78>
    80001b44:	8526                	mv	a0,s1
    80001b46:	f19ff0ef          	jal	80001a5e <freeproc>
    80001b4a:	8526                	mv	a0,s1
    80001b4c:	902ff0ef          	jal	80000c4e <release>
    80001b50:	84ca                	mv	s1,s2
    80001b52:	bfd1                	j	80001b26 <allocproc+0x78>

0000000080001b54 <userinit>:
    80001b54:	1101                	add	sp,sp,-32
    80001b56:	ec06                	sd	ra,24(sp)
    80001b58:	e822                	sd	s0,16(sp)
    80001b5a:	e426                	sd	s1,8(sp)
    80001b5c:	1000                	add	s0,sp,32
    80001b5e:	f51ff0ef          	jal	80001aae <allocproc>
    80001b62:	84aa                	mv	s1,a0
    80001b64:	00006797          	auipc	a5,0x6
    80001b68:	cea7b623          	sd	a0,-788(a5) # 80007850 <initproc>
    80001b6c:	00005517          	auipc	a0,0x5
    80001b70:	66c50513          	add	a0,a0,1644 # 800071d8 <states.1753+0x68>
    80001b74:	641010ef          	jal	800039b4 <namei>
    80001b78:	14a4b823          	sd	a0,336(s1)
    80001b7c:	478d                	li	a5,3
    80001b7e:	cc9c                	sw	a5,24(s1)
    80001b80:	8526                	mv	a0,s1
    80001b82:	8ccff0ef          	jal	80000c4e <release>
    80001b86:	60e2                	ld	ra,24(sp)
    80001b88:	6442                	ld	s0,16(sp)
    80001b8a:	64a2                	ld	s1,8(sp)
    80001b8c:	6105                	add	sp,sp,32
    80001b8e:	8082                	ret

0000000080001b90 <growproc>:
    80001b90:	1101                	add	sp,sp,-32
    80001b92:	ec06                	sd	ra,24(sp)
    80001b94:	e822                	sd	s0,16(sp)
    80001b96:	e426                	sd	s1,8(sp)
    80001b98:	e04a                	sd	s2,0(sp)
    80001b9a:	1000                	add	s0,sp,32
    80001b9c:	84aa                	mv	s1,a0
    80001b9e:	cefff0ef          	jal	8000188c <myproc>
    80001ba2:	892a                	mv	s2,a0
    80001ba4:	652c                	ld	a1,72(a0)
    80001ba6:	02905963          	blez	s1,80001bd8 <growproc+0x48>
    80001baa:	00b48633          	add	a2,s1,a1
    80001bae:	020007b7          	lui	a5,0x2000
    80001bb2:	17fd                	add	a5,a5,-1 # 1ffffff <_entry-0x7e000001>
    80001bb4:	07b6                	sll	a5,a5,0xd
    80001bb6:	02c7ea63          	bltu	a5,a2,80001bea <growproc+0x5a>
    80001bba:	4691                	li	a3,4
    80001bbc:	6928                	ld	a0,80(a0)
    80001bbe:	ed4ff0ef          	jal	80001292 <uvmalloc>
    80001bc2:	85aa                	mv	a1,a0
    80001bc4:	c50d                	beqz	a0,80001bee <growproc+0x5e>
    80001bc6:	04b93423          	sd	a1,72(s2)
    80001bca:	4501                	li	a0,0
    80001bcc:	60e2                	ld	ra,24(sp)
    80001bce:	6442                	ld	s0,16(sp)
    80001bd0:	64a2                	ld	s1,8(sp)
    80001bd2:	6902                	ld	s2,0(sp)
    80001bd4:	6105                	add	sp,sp,32
    80001bd6:	8082                	ret
    80001bd8:	fe04d7e3          	bgez	s1,80001bc6 <growproc+0x36>
    80001bdc:	00b48633          	add	a2,s1,a1
    80001be0:	6928                	ld	a0,80(a0)
    80001be2:	e6eff0ef          	jal	80001250 <uvmdealloc>
    80001be6:	85aa                	mv	a1,a0
    80001be8:	bff9                	j	80001bc6 <growproc+0x36>
    80001bea:	557d                	li	a0,-1
    80001bec:	b7c5                	j	80001bcc <growproc+0x3c>
    80001bee:	557d                	li	a0,-1
    80001bf0:	bff1                	j	80001bcc <growproc+0x3c>

0000000080001bf2 <kfork>:
    80001bf2:	7179                	add	sp,sp,-48
    80001bf4:	f406                	sd	ra,40(sp)
    80001bf6:	f022                	sd	s0,32(sp)
    80001bf8:	ec26                	sd	s1,24(sp)
    80001bfa:	e84a                	sd	s2,16(sp)
    80001bfc:	e44e                	sd	s3,8(sp)
    80001bfe:	e052                	sd	s4,0(sp)
    80001c00:	1800                	add	s0,sp,48
    80001c02:	c8bff0ef          	jal	8000188c <myproc>
    80001c06:	892a                	mv	s2,a0
    80001c08:	ea7ff0ef          	jal	80001aae <allocproc>
    80001c0c:	0e050563          	beqz	a0,80001cf6 <kfork+0x104>
    80001c10:	89aa                	mv	s3,a0
    80001c12:	04893603          	ld	a2,72(s2)
    80001c16:	692c                	ld	a1,80(a0)
    80001c18:	05093503          	ld	a0,80(s2)
    80001c1c:	f9eff0ef          	jal	800013ba <uvmcopy>
    80001c20:	04054663          	bltz	a0,80001c6c <kfork+0x7a>
    80001c24:	04893783          	ld	a5,72(s2)
    80001c28:	04f9b423          	sd	a5,72(s3)
    80001c2c:	05893683          	ld	a3,88(s2)
    80001c30:	87b6                	mv	a5,a3
    80001c32:	0589b703          	ld	a4,88(s3)
    80001c36:	12068693          	add	a3,a3,288
    80001c3a:	0007b803          	ld	a6,0(a5)
    80001c3e:	6788                	ld	a0,8(a5)
    80001c40:	6b8c                	ld	a1,16(a5)
    80001c42:	6f90                	ld	a2,24(a5)
    80001c44:	01073023          	sd	a6,0(a4) # 1000 <_entry-0x7ffff000>
    80001c48:	e708                	sd	a0,8(a4)
    80001c4a:	eb0c                	sd	a1,16(a4)
    80001c4c:	ef10                	sd	a2,24(a4)
    80001c4e:	02078793          	add	a5,a5,32
    80001c52:	02070713          	add	a4,a4,32
    80001c56:	fed792e3          	bne	a5,a3,80001c3a <kfork+0x48>
    80001c5a:	0589b783          	ld	a5,88(s3)
    80001c5e:	0607b823          	sd	zero,112(a5)
    80001c62:	0d000493          	li	s1,208
    80001c66:	15000a13          	li	s4,336
    80001c6a:	a00d                	j	80001c8c <kfork+0x9a>
    80001c6c:	854e                	mv	a0,s3
    80001c6e:	df1ff0ef          	jal	80001a5e <freeproc>
    80001c72:	854e                	mv	a0,s3
    80001c74:	fdbfe0ef          	jal	80000c4e <release>
    80001c78:	5a7d                	li	s4,-1
    80001c7a:	a0ad                	j	80001ce4 <kfork+0xf2>
    80001c7c:	324020ef          	jal	80003fa0 <filedup>
    80001c80:	009987b3          	add	a5,s3,s1
    80001c84:	e388                	sd	a0,0(a5)
    80001c86:	04a1                	add	s1,s1,8
    80001c88:	01448763          	beq	s1,s4,80001c96 <kfork+0xa4>
    80001c8c:	009907b3          	add	a5,s2,s1
    80001c90:	6388                	ld	a0,0(a5)
    80001c92:	f56d                	bnez	a0,80001c7c <kfork+0x8a>
    80001c94:	bfcd                	j	80001c86 <kfork+0x94>
    80001c96:	15093503          	ld	a0,336(s2)
    80001c9a:	4ea010ef          	jal	80003184 <idup>
    80001c9e:	14a9b823          	sd	a0,336(s3)
    80001ca2:	4641                	li	a2,16
    80001ca4:	15890593          	add	a1,s2,344
    80001ca8:	15898513          	add	a0,s3,344
    80001cac:	94eff0ef          	jal	80000dfa <safestrcpy>
    80001cb0:	0309aa03          	lw	s4,48(s3)
    80001cb4:	854e                	mv	a0,s3
    80001cb6:	f99fe0ef          	jal	80000c4e <release>
    80001cba:	0000e497          	auipc	s1,0xe
    80001cbe:	cb648493          	add	s1,s1,-842 # 8000f970 <wait_lock>
    80001cc2:	8526                	mv	a0,s1
    80001cc4:	ef3fe0ef          	jal	80000bb6 <acquire>
    80001cc8:	0329bc23          	sd	s2,56(s3)
    80001ccc:	8526                	mv	a0,s1
    80001cce:	f81fe0ef          	jal	80000c4e <release>
    80001cd2:	854e                	mv	a0,s3
    80001cd4:	ee3fe0ef          	jal	80000bb6 <acquire>
    80001cd8:	478d                	li	a5,3
    80001cda:	00f9ac23          	sw	a5,24(s3)
    80001cde:	854e                	mv	a0,s3
    80001ce0:	f6ffe0ef          	jal	80000c4e <release>
    80001ce4:	8552                	mv	a0,s4
    80001ce6:	70a2                	ld	ra,40(sp)
    80001ce8:	7402                	ld	s0,32(sp)
    80001cea:	64e2                	ld	s1,24(sp)
    80001cec:	6942                	ld	s2,16(sp)
    80001cee:	69a2                	ld	s3,8(sp)
    80001cf0:	6a02                	ld	s4,0(sp)
    80001cf2:	6145                	add	sp,sp,48
    80001cf4:	8082                	ret
    80001cf6:	5a7d                	li	s4,-1
    80001cf8:	b7f5                	j	80001ce4 <kfork+0xf2>

0000000080001cfa <scheduler>:
    80001cfa:	715d                	add	sp,sp,-80
    80001cfc:	e486                	sd	ra,72(sp)
    80001cfe:	e0a2                	sd	s0,64(sp)
    80001d00:	fc26                	sd	s1,56(sp)
    80001d02:	f84a                	sd	s2,48(sp)
    80001d04:	f44e                	sd	s3,40(sp)
    80001d06:	f052                	sd	s4,32(sp)
    80001d08:	ec56                	sd	s5,24(sp)
    80001d0a:	e85a                	sd	s6,16(sp)
    80001d0c:	e45e                	sd	s7,8(sp)
    80001d0e:	e062                	sd	s8,0(sp)
    80001d10:	0880                	add	s0,sp,80
    80001d12:	8792                	mv	a5,tp
    80001d14:	2781                	sext.w	a5,a5
    80001d16:	00779b13          	sll	s6,a5,0x7
    80001d1a:	0000e717          	auipc	a4,0xe
    80001d1e:	c3e70713          	add	a4,a4,-962 # 8000f958 <pid_lock>
    80001d22:	975a                	add	a4,a4,s6
    80001d24:	02073823          	sd	zero,48(a4)
    80001d28:	0000e717          	auipc	a4,0xe
    80001d2c:	c6870713          	add	a4,a4,-920 # 8000f990 <cpus+0x8>
    80001d30:	9b3a                	add	s6,s6,a4
    80001d32:	4c11                	li	s8,4
    80001d34:	079e                	sll	a5,a5,0x7
    80001d36:	0000ea17          	auipc	s4,0xe
    80001d3a:	c22a0a13          	add	s4,s4,-990 # 8000f958 <pid_lock>
    80001d3e:	9a3e                	add	s4,s4,a5
    80001d40:	00014997          	auipc	s3,0x14
    80001d44:	a4898993          	add	s3,s3,-1464 # 80015788 <tickslock>
    80001d48:	4b85                	li	s7,1
    80001d4a:	a83d                	j	80001d88 <scheduler+0x8e>
    80001d4c:	0184ac23          	sw	s8,24(s1)
    80001d50:	029a3823          	sd	s1,48(s4)
    80001d54:	06048593          	add	a1,s1,96
    80001d58:	855a                	mv	a0,s6
    80001d5a:	5d0000ef          	jal	8000232a <swtch>
    80001d5e:	020a3823          	sd	zero,48(s4)
    80001d62:	8ade                	mv	s5,s7
    80001d64:	8526                	mv	a0,s1
    80001d66:	ee9fe0ef          	jal	80000c4e <release>
    80001d6a:	16848493          	add	s1,s1,360
    80001d6e:	01348963          	beq	s1,s3,80001d80 <scheduler+0x86>
    80001d72:	8526                	mv	a0,s1
    80001d74:	e43fe0ef          	jal	80000bb6 <acquire>
    80001d78:	4c9c                	lw	a5,24(s1)
    80001d7a:	ff2795e3          	bne	a5,s2,80001d64 <scheduler+0x6a>
    80001d7e:	b7f9                	j	80001d4c <scheduler+0x52>
    80001d80:	000a9463          	bnez	s5,80001d88 <scheduler+0x8e>
    80001d84:	10500073          	wfi
    80001d88:	100027f3          	csrr	a5,sstatus
    80001d8c:	0027e793          	or	a5,a5,2
    80001d90:	10079073          	csrw	sstatus,a5
    80001d94:	100027f3          	csrr	a5,sstatus
    80001d98:	9bf5                	and	a5,a5,-3
    80001d9a:	10079073          	csrw	sstatus,a5
    80001d9e:	4a81                	li	s5,0
    80001da0:	0000e497          	auipc	s1,0xe
    80001da4:	fe848493          	add	s1,s1,-24 # 8000fd88 <proc>
    80001da8:	490d                	li	s2,3
    80001daa:	b7e1                	j	80001d72 <scheduler+0x78>

0000000080001dac <sched>:
    80001dac:	7179                	add	sp,sp,-48
    80001dae:	f406                	sd	ra,40(sp)
    80001db0:	f022                	sd	s0,32(sp)
    80001db2:	ec26                	sd	s1,24(sp)
    80001db4:	e84a                	sd	s2,16(sp)
    80001db6:	e44e                	sd	s3,8(sp)
    80001db8:	1800                	add	s0,sp,48
    80001dba:	ad3ff0ef          	jal	8000188c <myproc>
    80001dbe:	892a                	mv	s2,a0
    80001dc0:	d8dfe0ef          	jal	80000b4c <holding>
    80001dc4:	c935                	beqz	a0,80001e38 <sched+0x8c>
    80001dc6:	8792                	mv	a5,tp
    80001dc8:	2781                	sext.w	a5,a5
    80001dca:	079e                	sll	a5,a5,0x7
    80001dcc:	0000e717          	auipc	a4,0xe
    80001dd0:	b8c70713          	add	a4,a4,-1140 # 8000f958 <pid_lock>
    80001dd4:	97ba                	add	a5,a5,a4
    80001dd6:	0a87a703          	lw	a4,168(a5)
    80001dda:	4785                	li	a5,1
    80001ddc:	06f71463          	bne	a4,a5,80001e44 <sched+0x98>
    80001de0:	01892703          	lw	a4,24(s2)
    80001de4:	4791                	li	a5,4
    80001de6:	06f70563          	beq	a4,a5,80001e50 <sched+0xa4>
    80001dea:	100027f3          	csrr	a5,sstatus
    80001dee:	8b89                	and	a5,a5,2
    80001df0:	e7b5                	bnez	a5,80001e5c <sched+0xb0>
    80001df2:	8792                	mv	a5,tp
    80001df4:	0000e497          	auipc	s1,0xe
    80001df8:	b6448493          	add	s1,s1,-1180 # 8000f958 <pid_lock>
    80001dfc:	2781                	sext.w	a5,a5
    80001dfe:	079e                	sll	a5,a5,0x7
    80001e00:	97a6                	add	a5,a5,s1
    80001e02:	0ac7a983          	lw	s3,172(a5)
    80001e06:	8792                	mv	a5,tp
    80001e08:	2781                	sext.w	a5,a5
    80001e0a:	079e                	sll	a5,a5,0x7
    80001e0c:	0000e597          	auipc	a1,0xe
    80001e10:	b8458593          	add	a1,a1,-1148 # 8000f990 <cpus+0x8>
    80001e14:	95be                	add	a1,a1,a5
    80001e16:	06090513          	add	a0,s2,96
    80001e1a:	510000ef          	jal	8000232a <swtch>
    80001e1e:	8792                	mv	a5,tp
    80001e20:	2781                	sext.w	a5,a5
    80001e22:	079e                	sll	a5,a5,0x7
    80001e24:	97a6                	add	a5,a5,s1
    80001e26:	0b37a623          	sw	s3,172(a5)
    80001e2a:	70a2                	ld	ra,40(sp)
    80001e2c:	7402                	ld	s0,32(sp)
    80001e2e:	64e2                	ld	s1,24(sp)
    80001e30:	6942                	ld	s2,16(sp)
    80001e32:	69a2                	ld	s3,8(sp)
    80001e34:	6145                	add	sp,sp,48
    80001e36:	8082                	ret
    80001e38:	00005517          	auipc	a0,0x5
    80001e3c:	3a850513          	add	a0,a0,936 # 800071e0 <states.1753+0x70>
    80001e40:	983fe0ef          	jal	800007c2 <panic>
    80001e44:	00005517          	auipc	a0,0x5
    80001e48:	3ac50513          	add	a0,a0,940 # 800071f0 <states.1753+0x80>
    80001e4c:	977fe0ef          	jal	800007c2 <panic>
    80001e50:	00005517          	auipc	a0,0x5
    80001e54:	3b050513          	add	a0,a0,944 # 80007200 <states.1753+0x90>
    80001e58:	96bfe0ef          	jal	800007c2 <panic>
    80001e5c:	00005517          	auipc	a0,0x5
    80001e60:	3b450513          	add	a0,a0,948 # 80007210 <states.1753+0xa0>
    80001e64:	95ffe0ef          	jal	800007c2 <panic>

0000000080001e68 <yield>:
    80001e68:	1101                	add	sp,sp,-32
    80001e6a:	ec06                	sd	ra,24(sp)
    80001e6c:	e822                	sd	s0,16(sp)
    80001e6e:	e426                	sd	s1,8(sp)
    80001e70:	1000                	add	s0,sp,32
    80001e72:	a1bff0ef          	jal	8000188c <myproc>
    80001e76:	84aa                	mv	s1,a0
    80001e78:	d3ffe0ef          	jal	80000bb6 <acquire>
    80001e7c:	478d                	li	a5,3
    80001e7e:	cc9c                	sw	a5,24(s1)
    80001e80:	f2dff0ef          	jal	80001dac <sched>
    80001e84:	8526                	mv	a0,s1
    80001e86:	dc9fe0ef          	jal	80000c4e <release>
    80001e8a:	60e2                	ld	ra,24(sp)
    80001e8c:	6442                	ld	s0,16(sp)
    80001e8e:	64a2                	ld	s1,8(sp)
    80001e90:	6105                	add	sp,sp,32
    80001e92:	8082                	ret

0000000080001e94 <sleep>:
    80001e94:	7179                	add	sp,sp,-48
    80001e96:	f406                	sd	ra,40(sp)
    80001e98:	f022                	sd	s0,32(sp)
    80001e9a:	ec26                	sd	s1,24(sp)
    80001e9c:	e84a                	sd	s2,16(sp)
    80001e9e:	e44e                	sd	s3,8(sp)
    80001ea0:	1800                	add	s0,sp,48
    80001ea2:	89aa                	mv	s3,a0
    80001ea4:	892e                	mv	s2,a1
    80001ea6:	9e7ff0ef          	jal	8000188c <myproc>
    80001eaa:	84aa                	mv	s1,a0
    80001eac:	d0bfe0ef          	jal	80000bb6 <acquire>
    80001eb0:	854a                	mv	a0,s2
    80001eb2:	d9dfe0ef          	jal	80000c4e <release>
    80001eb6:	0334b023          	sd	s3,32(s1)
    80001eba:	4789                	li	a5,2
    80001ebc:	cc9c                	sw	a5,24(s1)
    80001ebe:	eefff0ef          	jal	80001dac <sched>
    80001ec2:	0204b023          	sd	zero,32(s1)
    80001ec6:	8526                	mv	a0,s1
    80001ec8:	d87fe0ef          	jal	80000c4e <release>
    80001ecc:	854a                	mv	a0,s2
    80001ece:	ce9fe0ef          	jal	80000bb6 <acquire>
    80001ed2:	70a2                	ld	ra,40(sp)
    80001ed4:	7402                	ld	s0,32(sp)
    80001ed6:	64e2                	ld	s1,24(sp)
    80001ed8:	6942                	ld	s2,16(sp)
    80001eda:	69a2                	ld	s3,8(sp)
    80001edc:	6145                	add	sp,sp,48
    80001ede:	8082                	ret

0000000080001ee0 <wakeup>:
    80001ee0:	7139                	add	sp,sp,-64
    80001ee2:	fc06                	sd	ra,56(sp)
    80001ee4:	f822                	sd	s0,48(sp)
    80001ee6:	f426                	sd	s1,40(sp)
    80001ee8:	f04a                	sd	s2,32(sp)
    80001eea:	ec4e                	sd	s3,24(sp)
    80001eec:	e852                	sd	s4,16(sp)
    80001eee:	e456                	sd	s5,8(sp)
    80001ef0:	0080                	add	s0,sp,64
    80001ef2:	8a2a                	mv	s4,a0
    80001ef4:	0000e497          	auipc	s1,0xe
    80001ef8:	e9448493          	add	s1,s1,-364 # 8000fd88 <proc>
    80001efc:	4989                	li	s3,2
    80001efe:	4a8d                	li	s5,3
    80001f00:	00014917          	auipc	s2,0x14
    80001f04:	88890913          	add	s2,s2,-1912 # 80015788 <tickslock>
    80001f08:	a811                	j	80001f1c <wakeup+0x3c>
    80001f0a:	0154ac23          	sw	s5,24(s1)
    80001f0e:	8526                	mv	a0,s1
    80001f10:	d3ffe0ef          	jal	80000c4e <release>
    80001f14:	16848493          	add	s1,s1,360
    80001f18:	03248063          	beq	s1,s2,80001f38 <wakeup+0x58>
    80001f1c:	971ff0ef          	jal	8000188c <myproc>
    80001f20:	fea48ae3          	beq	s1,a0,80001f14 <wakeup+0x34>
    80001f24:	8526                	mv	a0,s1
    80001f26:	c91fe0ef          	jal	80000bb6 <acquire>
    80001f2a:	4c9c                	lw	a5,24(s1)
    80001f2c:	ff3791e3          	bne	a5,s3,80001f0e <wakeup+0x2e>
    80001f30:	709c                	ld	a5,32(s1)
    80001f32:	fd479ee3          	bne	a5,s4,80001f0e <wakeup+0x2e>
    80001f36:	bfd1                	j	80001f0a <wakeup+0x2a>
    80001f38:	70e2                	ld	ra,56(sp)
    80001f3a:	7442                	ld	s0,48(sp)
    80001f3c:	74a2                	ld	s1,40(sp)
    80001f3e:	7902                	ld	s2,32(sp)
    80001f40:	69e2                	ld	s3,24(sp)
    80001f42:	6a42                	ld	s4,16(sp)
    80001f44:	6aa2                	ld	s5,8(sp)
    80001f46:	6121                	add	sp,sp,64
    80001f48:	8082                	ret

0000000080001f4a <reparent>:
    80001f4a:	7179                	add	sp,sp,-48
    80001f4c:	f406                	sd	ra,40(sp)
    80001f4e:	f022                	sd	s0,32(sp)
    80001f50:	ec26                	sd	s1,24(sp)
    80001f52:	e84a                	sd	s2,16(sp)
    80001f54:	e44e                	sd	s3,8(sp)
    80001f56:	e052                	sd	s4,0(sp)
    80001f58:	1800                	add	s0,sp,48
    80001f5a:	89aa                	mv	s3,a0
    80001f5c:	0000e497          	auipc	s1,0xe
    80001f60:	e2c48493          	add	s1,s1,-468 # 8000fd88 <proc>
    80001f64:	00006a17          	auipc	s4,0x6
    80001f68:	8eca0a13          	add	s4,s4,-1812 # 80007850 <initproc>
    80001f6c:	00014917          	auipc	s2,0x14
    80001f70:	81c90913          	add	s2,s2,-2020 # 80015788 <tickslock>
    80001f74:	a029                	j	80001f7e <reparent+0x34>
    80001f76:	16848493          	add	s1,s1,360
    80001f7a:	01248b63          	beq	s1,s2,80001f90 <reparent+0x46>
    80001f7e:	7c9c                	ld	a5,56(s1)
    80001f80:	ff379be3          	bne	a5,s3,80001f76 <reparent+0x2c>
    80001f84:	000a3503          	ld	a0,0(s4)
    80001f88:	fc88                	sd	a0,56(s1)
    80001f8a:	f57ff0ef          	jal	80001ee0 <wakeup>
    80001f8e:	b7e5                	j	80001f76 <reparent+0x2c>
    80001f90:	70a2                	ld	ra,40(sp)
    80001f92:	7402                	ld	s0,32(sp)
    80001f94:	64e2                	ld	s1,24(sp)
    80001f96:	6942                	ld	s2,16(sp)
    80001f98:	69a2                	ld	s3,8(sp)
    80001f9a:	6a02                	ld	s4,0(sp)
    80001f9c:	6145                	add	sp,sp,48
    80001f9e:	8082                	ret

0000000080001fa0 <kexit>:
    80001fa0:	7179                	add	sp,sp,-48
    80001fa2:	f406                	sd	ra,40(sp)
    80001fa4:	f022                	sd	s0,32(sp)
    80001fa6:	ec26                	sd	s1,24(sp)
    80001fa8:	e84a                	sd	s2,16(sp)
    80001faa:	e44e                	sd	s3,8(sp)
    80001fac:	e052                	sd	s4,0(sp)
    80001fae:	1800                	add	s0,sp,48
    80001fb0:	8a2a                	mv	s4,a0
    80001fb2:	8dbff0ef          	jal	8000188c <myproc>
    80001fb6:	89aa                	mv	s3,a0
    80001fb8:	00006797          	auipc	a5,0x6
    80001fbc:	89878793          	add	a5,a5,-1896 # 80007850 <initproc>
    80001fc0:	639c                	ld	a5,0(a5)
    80001fc2:	0d050493          	add	s1,a0,208
    80001fc6:	15050913          	add	s2,a0,336
    80001fca:	00a79f63          	bne	a5,a0,80001fe8 <kexit+0x48>
    80001fce:	00005517          	auipc	a0,0x5
    80001fd2:	25a50513          	add	a0,a0,602 # 80007228 <states.1753+0xb8>
    80001fd6:	fecfe0ef          	jal	800007c2 <panic>
    80001fda:	00c020ef          	jal	80003fe6 <fileclose>
    80001fde:	0004b023          	sd	zero,0(s1)
    80001fe2:	04a1                	add	s1,s1,8
    80001fe4:	01248563          	beq	s1,s2,80001fee <kexit+0x4e>
    80001fe8:	6088                	ld	a0,0(s1)
    80001fea:	f965                	bnez	a0,80001fda <kexit+0x3a>
    80001fec:	bfdd                	j	80001fe2 <kexit+0x42>
    80001fee:	3b7010ef          	jal	80003ba4 <begin_op>
    80001ff2:	1509b503          	ld	a0,336(s3)
    80001ff6:	344010ef          	jal	8000333a <iput>
    80001ffa:	41b010ef          	jal	80003c14 <end_op>
    80001ffe:	1409b823          	sd	zero,336(s3)
    80002002:	0000e497          	auipc	s1,0xe
    80002006:	96e48493          	add	s1,s1,-1682 # 8000f970 <wait_lock>
    8000200a:	8526                	mv	a0,s1
    8000200c:	babfe0ef          	jal	80000bb6 <acquire>
    80002010:	854e                	mv	a0,s3
    80002012:	f39ff0ef          	jal	80001f4a <reparent>
    80002016:	0389b503          	ld	a0,56(s3)
    8000201a:	ec7ff0ef          	jal	80001ee0 <wakeup>
    8000201e:	854e                	mv	a0,s3
    80002020:	b97fe0ef          	jal	80000bb6 <acquire>
    80002024:	0349a623          	sw	s4,44(s3)
    80002028:	4795                	li	a5,5
    8000202a:	00f9ac23          	sw	a5,24(s3)
    8000202e:	8526                	mv	a0,s1
    80002030:	c1ffe0ef          	jal	80000c4e <release>
    80002034:	d79ff0ef          	jal	80001dac <sched>
    80002038:	00005517          	auipc	a0,0x5
    8000203c:	20050513          	add	a0,a0,512 # 80007238 <states.1753+0xc8>
    80002040:	f82fe0ef          	jal	800007c2 <panic>

0000000080002044 <kkill>:
    80002044:	7179                	add	sp,sp,-48
    80002046:	f406                	sd	ra,40(sp)
    80002048:	f022                	sd	s0,32(sp)
    8000204a:	ec26                	sd	s1,24(sp)
    8000204c:	e84a                	sd	s2,16(sp)
    8000204e:	e44e                	sd	s3,8(sp)
    80002050:	1800                	add	s0,sp,48
    80002052:	892a                	mv	s2,a0
    80002054:	0000e497          	auipc	s1,0xe
    80002058:	d3448493          	add	s1,s1,-716 # 8000fd88 <proc>
    8000205c:	00013997          	auipc	s3,0x13
    80002060:	72c98993          	add	s3,s3,1836 # 80015788 <tickslock>
    80002064:	8526                	mv	a0,s1
    80002066:	b51fe0ef          	jal	80000bb6 <acquire>
    8000206a:	589c                	lw	a5,48(s1)
    8000206c:	01278b63          	beq	a5,s2,80002082 <kkill+0x3e>
    80002070:	8526                	mv	a0,s1
    80002072:	bddfe0ef          	jal	80000c4e <release>
    80002076:	16848493          	add	s1,s1,360
    8000207a:	ff3495e3          	bne	s1,s3,80002064 <kkill+0x20>
    8000207e:	557d                	li	a0,-1
    80002080:	a819                	j	80002096 <kkill+0x52>
    80002082:	4785                	li	a5,1
    80002084:	d49c                	sw	a5,40(s1)
    80002086:	4c98                	lw	a4,24(s1)
    80002088:	4789                	li	a5,2
    8000208a:	00f70d63          	beq	a4,a5,800020a4 <kkill+0x60>
    8000208e:	8526                	mv	a0,s1
    80002090:	bbffe0ef          	jal	80000c4e <release>
    80002094:	4501                	li	a0,0
    80002096:	70a2                	ld	ra,40(sp)
    80002098:	7402                	ld	s0,32(sp)
    8000209a:	64e2                	ld	s1,24(sp)
    8000209c:	6942                	ld	s2,16(sp)
    8000209e:	69a2                	ld	s3,8(sp)
    800020a0:	6145                	add	sp,sp,48
    800020a2:	8082                	ret
    800020a4:	478d                	li	a5,3
    800020a6:	cc9c                	sw	a5,24(s1)
    800020a8:	b7dd                	j	8000208e <kkill+0x4a>

00000000800020aa <setkilled>:
    800020aa:	1101                	add	sp,sp,-32
    800020ac:	ec06                	sd	ra,24(sp)
    800020ae:	e822                	sd	s0,16(sp)
    800020b0:	e426                	sd	s1,8(sp)
    800020b2:	1000                	add	s0,sp,32
    800020b4:	84aa                	mv	s1,a0
    800020b6:	b01fe0ef          	jal	80000bb6 <acquire>
    800020ba:	4785                	li	a5,1
    800020bc:	d49c                	sw	a5,40(s1)
    800020be:	8526                	mv	a0,s1
    800020c0:	b8ffe0ef          	jal	80000c4e <release>
    800020c4:	60e2                	ld	ra,24(sp)
    800020c6:	6442                	ld	s0,16(sp)
    800020c8:	64a2                	ld	s1,8(sp)
    800020ca:	6105                	add	sp,sp,32
    800020cc:	8082                	ret

00000000800020ce <killed>:
    800020ce:	1101                	add	sp,sp,-32
    800020d0:	ec06                	sd	ra,24(sp)
    800020d2:	e822                	sd	s0,16(sp)
    800020d4:	e426                	sd	s1,8(sp)
    800020d6:	e04a                	sd	s2,0(sp)
    800020d8:	1000                	add	s0,sp,32
    800020da:	84aa                	mv	s1,a0
    800020dc:	adbfe0ef          	jal	80000bb6 <acquire>
    800020e0:	0284a903          	lw	s2,40(s1)
    800020e4:	8526                	mv	a0,s1
    800020e6:	b69fe0ef          	jal	80000c4e <release>
    800020ea:	854a                	mv	a0,s2
    800020ec:	60e2                	ld	ra,24(sp)
    800020ee:	6442                	ld	s0,16(sp)
    800020f0:	64a2                	ld	s1,8(sp)
    800020f2:	6902                	ld	s2,0(sp)
    800020f4:	6105                	add	sp,sp,32
    800020f6:	8082                	ret

00000000800020f8 <kwait>:
    800020f8:	715d                	add	sp,sp,-80
    800020fa:	e486                	sd	ra,72(sp)
    800020fc:	e0a2                	sd	s0,64(sp)
    800020fe:	fc26                	sd	s1,56(sp)
    80002100:	f84a                	sd	s2,48(sp)
    80002102:	f44e                	sd	s3,40(sp)
    80002104:	f052                	sd	s4,32(sp)
    80002106:	ec56                	sd	s5,24(sp)
    80002108:	e85a                	sd	s6,16(sp)
    8000210a:	e45e                	sd	s7,8(sp)
    8000210c:	e062                	sd	s8,0(sp)
    8000210e:	0880                	add	s0,sp,80
    80002110:	8baa                	mv	s7,a0
    80002112:	f7aff0ef          	jal	8000188c <myproc>
    80002116:	892a                	mv	s2,a0
    80002118:	0000e517          	auipc	a0,0xe
    8000211c:	85850513          	add	a0,a0,-1960 # 8000f970 <wait_lock>
    80002120:	a97fe0ef          	jal	80000bb6 <acquire>
    80002124:	4b01                	li	s6,0
    80002126:	4a15                	li	s4,5
    80002128:	00013997          	auipc	s3,0x13
    8000212c:	66098993          	add	s3,s3,1632 # 80015788 <tickslock>
    80002130:	4a85                	li	s5,1
    80002132:	0000ec17          	auipc	s8,0xe
    80002136:	83ec0c13          	add	s8,s8,-1986 # 8000f970 <wait_lock>
    8000213a:	875a                	mv	a4,s6
    8000213c:	0000e497          	auipc	s1,0xe
    80002140:	c4c48493          	add	s1,s1,-948 # 8000fd88 <proc>
    80002144:	a899                	j	8000219a <kwait+0xa2>
    80002146:	0304a983          	lw	s3,48(s1)
    8000214a:	000b8c63          	beqz	s7,80002162 <kwait+0x6a>
    8000214e:	4691                	li	a3,4
    80002150:	02c48613          	add	a2,s1,44
    80002154:	85de                	mv	a1,s7
    80002156:	05093503          	ld	a0,80(s2)
    8000215a:	c7cff0ef          	jal	800015d6 <copyout>
    8000215e:	00054f63          	bltz	a0,8000217c <kwait+0x84>
    80002162:	8526                	mv	a0,s1
    80002164:	8fbff0ef          	jal	80001a5e <freeproc>
    80002168:	8526                	mv	a0,s1
    8000216a:	ae5fe0ef          	jal	80000c4e <release>
    8000216e:	0000e517          	auipc	a0,0xe
    80002172:	80250513          	add	a0,a0,-2046 # 8000f970 <wait_lock>
    80002176:	ad9fe0ef          	jal	80000c4e <release>
    8000217a:	a891                	j	800021ce <kwait+0xd6>
    8000217c:	8526                	mv	a0,s1
    8000217e:	ad1fe0ef          	jal	80000c4e <release>
    80002182:	0000d517          	auipc	a0,0xd
    80002186:	7ee50513          	add	a0,a0,2030 # 8000f970 <wait_lock>
    8000218a:	ac5fe0ef          	jal	80000c4e <release>
    8000218e:	59fd                	li	s3,-1
    80002190:	a83d                	j	800021ce <kwait+0xd6>
    80002192:	16848493          	add	s1,s1,360
    80002196:	03348063          	beq	s1,s3,800021b6 <kwait+0xbe>
    8000219a:	7c9c                	ld	a5,56(s1)
    8000219c:	ff279be3          	bne	a5,s2,80002192 <kwait+0x9a>
    800021a0:	8526                	mv	a0,s1
    800021a2:	a15fe0ef          	jal	80000bb6 <acquire>
    800021a6:	4c9c                	lw	a5,24(s1)
    800021a8:	f9478fe3          	beq	a5,s4,80002146 <kwait+0x4e>
    800021ac:	8526                	mv	a0,s1
    800021ae:	aa1fe0ef          	jal	80000c4e <release>
    800021b2:	8756                	mv	a4,s5
    800021b4:	bff9                	j	80002192 <kwait+0x9a>
    800021b6:	c709                	beqz	a4,800021c0 <kwait+0xc8>
    800021b8:	854a                	mv	a0,s2
    800021ba:	f15ff0ef          	jal	800020ce <killed>
    800021be:	c50d                	beqz	a0,800021e8 <kwait+0xf0>
    800021c0:	0000d517          	auipc	a0,0xd
    800021c4:	7b050513          	add	a0,a0,1968 # 8000f970 <wait_lock>
    800021c8:	a87fe0ef          	jal	80000c4e <release>
    800021cc:	59fd                	li	s3,-1
    800021ce:	854e                	mv	a0,s3
    800021d0:	60a6                	ld	ra,72(sp)
    800021d2:	6406                	ld	s0,64(sp)
    800021d4:	74e2                	ld	s1,56(sp)
    800021d6:	7942                	ld	s2,48(sp)
    800021d8:	79a2                	ld	s3,40(sp)
    800021da:	7a02                	ld	s4,32(sp)
    800021dc:	6ae2                	ld	s5,24(sp)
    800021de:	6b42                	ld	s6,16(sp)
    800021e0:	6ba2                	ld	s7,8(sp)
    800021e2:	6c02                	ld	s8,0(sp)
    800021e4:	6161                	add	sp,sp,80
    800021e6:	8082                	ret
    800021e8:	85e2                	mv	a1,s8
    800021ea:	854a                	mv	a0,s2
    800021ec:	ca9ff0ef          	jal	80001e94 <sleep>
    800021f0:	b7a9                	j	8000213a <kwait+0x42>

00000000800021f2 <either_copyout>:
    800021f2:	7179                	add	sp,sp,-48
    800021f4:	f406                	sd	ra,40(sp)
    800021f6:	f022                	sd	s0,32(sp)
    800021f8:	ec26                	sd	s1,24(sp)
    800021fa:	e84a                	sd	s2,16(sp)
    800021fc:	e44e                	sd	s3,8(sp)
    800021fe:	e052                	sd	s4,0(sp)
    80002200:	1800                	add	s0,sp,48
    80002202:	84aa                	mv	s1,a0
    80002204:	892e                	mv	s2,a1
    80002206:	89b2                	mv	s3,a2
    80002208:	8a36                	mv	s4,a3
    8000220a:	e82ff0ef          	jal	8000188c <myproc>
    8000220e:	cc99                	beqz	s1,8000222c <either_copyout+0x3a>
    80002210:	86d2                	mv	a3,s4
    80002212:	864e                	mv	a2,s3
    80002214:	85ca                	mv	a1,s2
    80002216:	6928                	ld	a0,80(a0)
    80002218:	bbeff0ef          	jal	800015d6 <copyout>
    8000221c:	70a2                	ld	ra,40(sp)
    8000221e:	7402                	ld	s0,32(sp)
    80002220:	64e2                	ld	s1,24(sp)
    80002222:	6942                	ld	s2,16(sp)
    80002224:	69a2                	ld	s3,8(sp)
    80002226:	6a02                	ld	s4,0(sp)
    80002228:	6145                	add	sp,sp,48
    8000222a:	8082                	ret
    8000222c:	000a061b          	sext.w	a2,s4
    80002230:	85ce                	mv	a1,s3
    80002232:	854a                	mv	a0,s2
    80002234:	ac3fe0ef          	jal	80000cf6 <memmove>
    80002238:	8526                	mv	a0,s1
    8000223a:	b7cd                	j	8000221c <either_copyout+0x2a>

000000008000223c <either_copyin>:
    8000223c:	7179                	add	sp,sp,-48
    8000223e:	f406                	sd	ra,40(sp)
    80002240:	f022                	sd	s0,32(sp)
    80002242:	ec26                	sd	s1,24(sp)
    80002244:	e84a                	sd	s2,16(sp)
    80002246:	e44e                	sd	s3,8(sp)
    80002248:	e052                	sd	s4,0(sp)
    8000224a:	1800                	add	s0,sp,48
    8000224c:	892a                	mv	s2,a0
    8000224e:	84ae                	mv	s1,a1
    80002250:	89b2                	mv	s3,a2
    80002252:	8a36                	mv	s4,a3
    80002254:	e38ff0ef          	jal	8000188c <myproc>
    80002258:	cc99                	beqz	s1,80002276 <either_copyin+0x3a>
    8000225a:	86d2                	mv	a3,s4
    8000225c:	864e                	mv	a2,s3
    8000225e:	85ca                	mv	a1,s2
    80002260:	6928                	ld	a0,80(a0)
    80002262:	c3aff0ef          	jal	8000169c <copyin>
    80002266:	70a2                	ld	ra,40(sp)
    80002268:	7402                	ld	s0,32(sp)
    8000226a:	64e2                	ld	s1,24(sp)
    8000226c:	6942                	ld	s2,16(sp)
    8000226e:	69a2                	ld	s3,8(sp)
    80002270:	6a02                	ld	s4,0(sp)
    80002272:	6145                	add	sp,sp,48
    80002274:	8082                	ret
    80002276:	000a061b          	sext.w	a2,s4
    8000227a:	85ce                	mv	a1,s3
    8000227c:	854a                	mv	a0,s2
    8000227e:	a79fe0ef          	jal	80000cf6 <memmove>
    80002282:	8526                	mv	a0,s1
    80002284:	b7cd                	j	80002266 <either_copyin+0x2a>

0000000080002286 <procdump>:
    80002286:	715d                	add	sp,sp,-80
    80002288:	e486                	sd	ra,72(sp)
    8000228a:	e0a2                	sd	s0,64(sp)
    8000228c:	fc26                	sd	s1,56(sp)
    8000228e:	f84a                	sd	s2,48(sp)
    80002290:	f44e                	sd	s3,40(sp)
    80002292:	f052                	sd	s4,32(sp)
    80002294:	ec56                	sd	s5,24(sp)
    80002296:	e85a                	sd	s6,16(sp)
    80002298:	e45e                	sd	s7,8(sp)
    8000229a:	0880                	add	s0,sp,80
    8000229c:	00005517          	auipc	a0,0x5
    800022a0:	e2450513          	add	a0,a0,-476 # 800070c0 <digits+0xa8>
    800022a4:	a50fe0ef          	jal	800004f4 <printf>
    800022a8:	0000e497          	auipc	s1,0xe
    800022ac:	c3848493          	add	s1,s1,-968 # 8000fee0 <proc+0x158>
    800022b0:	00013917          	auipc	s2,0x13
    800022b4:	63090913          	add	s2,s2,1584 # 800158e0 <bcache+0x140>
    800022b8:	4b15                	li	s6,5
    800022ba:	00005997          	auipc	s3,0x5
    800022be:	f8e98993          	add	s3,s3,-114 # 80007248 <states.1753+0xd8>
    800022c2:	00005a97          	auipc	s5,0x5
    800022c6:	f8ea8a93          	add	s5,s5,-114 # 80007250 <states.1753+0xe0>
    800022ca:	00005a17          	auipc	s4,0x5
    800022ce:	df6a0a13          	add	s4,s4,-522 # 800070c0 <digits+0xa8>
    800022d2:	00005b97          	auipc	s7,0x5
    800022d6:	e9eb8b93          	add	s7,s7,-354 # 80007170 <states.1753>
    800022da:	a831                	j	800022f6 <procdump+0x70>
    800022dc:	86ba                	mv	a3,a4
    800022de:	ed872583          	lw	a1,-296(a4)
    800022e2:	8556                	mv	a0,s5
    800022e4:	a10fe0ef          	jal	800004f4 <printf>
    800022e8:	8552                	mv	a0,s4
    800022ea:	a0afe0ef          	jal	800004f4 <printf>
    800022ee:	16848493          	add	s1,s1,360
    800022f2:	03248163          	beq	s1,s2,80002314 <procdump+0x8e>
    800022f6:	8726                	mv	a4,s1
    800022f8:	ec04a783          	lw	a5,-320(s1)
    800022fc:	dbed                	beqz	a5,800022ee <procdump+0x68>
    800022fe:	864e                	mv	a2,s3
    80002300:	fcfb6ee3          	bltu	s6,a5,800022dc <procdump+0x56>
    80002304:	1782                	sll	a5,a5,0x20
    80002306:	9381                	srl	a5,a5,0x20
    80002308:	078e                	sll	a5,a5,0x3
    8000230a:	97de                	add	a5,a5,s7
    8000230c:	6390                	ld	a2,0(a5)
    8000230e:	f679                	bnez	a2,800022dc <procdump+0x56>
    80002310:	864e                	mv	a2,s3
    80002312:	b7e9                	j	800022dc <procdump+0x56>
    80002314:	60a6                	ld	ra,72(sp)
    80002316:	6406                	ld	s0,64(sp)
    80002318:	74e2                	ld	s1,56(sp)
    8000231a:	7942                	ld	s2,48(sp)
    8000231c:	79a2                	ld	s3,40(sp)
    8000231e:	7a02                	ld	s4,32(sp)
    80002320:	6ae2                	ld	s5,24(sp)
    80002322:	6b42                	ld	s6,16(sp)
    80002324:	6ba2                	ld	s7,8(sp)
    80002326:	6161                	add	sp,sp,80
    80002328:	8082                	ret

000000008000232a <swtch>:
    8000232a:	00153023          	sd	ra,0(a0)
    8000232e:	00253423          	sd	sp,8(a0)
    80002332:	e900                	sd	s0,16(a0)
    80002334:	ed04                	sd	s1,24(a0)
    80002336:	03253023          	sd	s2,32(a0)
    8000233a:	03353423          	sd	s3,40(a0)
    8000233e:	03453823          	sd	s4,48(a0)
    80002342:	03553c23          	sd	s5,56(a0)
    80002346:	05653023          	sd	s6,64(a0)
    8000234a:	05753423          	sd	s7,72(a0)
    8000234e:	05853823          	sd	s8,80(a0)
    80002352:	05953c23          	sd	s9,88(a0)
    80002356:	07a53023          	sd	s10,96(a0)
    8000235a:	07b53423          	sd	s11,104(a0)
    8000235e:	0005b083          	ld	ra,0(a1)
    80002362:	0085b103          	ld	sp,8(a1)
    80002366:	6980                	ld	s0,16(a1)
    80002368:	6d84                	ld	s1,24(a1)
    8000236a:	0205b903          	ld	s2,32(a1)
    8000236e:	0285b983          	ld	s3,40(a1)
    80002372:	0305ba03          	ld	s4,48(a1)
    80002376:	0385ba83          	ld	s5,56(a1)
    8000237a:	0405bb03          	ld	s6,64(a1)
    8000237e:	0485bb83          	ld	s7,72(a1)
    80002382:	0505bc03          	ld	s8,80(a1)
    80002386:	0585bc83          	ld	s9,88(a1)
    8000238a:	0605bd03          	ld	s10,96(a1)
    8000238e:	0685bd83          	ld	s11,104(a1)
    80002392:	8082                	ret

0000000080002394 <trapinit>:
    80002394:	1141                	add	sp,sp,-16
    80002396:	e406                	sd	ra,8(sp)
    80002398:	e022                	sd	s0,0(sp)
    8000239a:	0800                	add	s0,sp,16
    8000239c:	00005597          	auipc	a1,0x5
    800023a0:	ef458593          	add	a1,a1,-268 # 80007290 <states.1753+0x120>
    800023a4:	00013517          	auipc	a0,0x13
    800023a8:	3e450513          	add	a0,a0,996 # 80015788 <tickslock>
    800023ac:	f8afe0ef          	jal	80000b36 <initlock>
    800023b0:	60a2                	ld	ra,8(sp)
    800023b2:	6402                	ld	s0,0(sp)
    800023b4:	0141                	add	sp,sp,16
    800023b6:	8082                	ret

00000000800023b8 <trapinithart>:
    800023b8:	1141                	add	sp,sp,-16
    800023ba:	e422                	sd	s0,8(sp)
    800023bc:	0800                	add	s0,sp,16
    800023be:	00003797          	auipc	a5,0x3
    800023c2:	f2278793          	add	a5,a5,-222 # 800052e0 <kernelvec>
    800023c6:	10579073          	csrw	stvec,a5
    800023ca:	6422                	ld	s0,8(sp)
    800023cc:	0141                	add	sp,sp,16
    800023ce:	8082                	ret

00000000800023d0 <prepare_return>:
    800023d0:	1141                	add	sp,sp,-16
    800023d2:	e406                	sd	ra,8(sp)
    800023d4:	e022                	sd	s0,0(sp)
    800023d6:	0800                	add	s0,sp,16
    800023d8:	cb4ff0ef          	jal	8000188c <myproc>
    800023dc:	100027f3          	csrr	a5,sstatus
    800023e0:	9bf5                	and	a5,a5,-3
    800023e2:	10079073          	csrw	sstatus,a5
    800023e6:	04000737          	lui	a4,0x4000
    800023ea:	00004797          	auipc	a5,0x4
    800023ee:	c1678793          	add	a5,a5,-1002 # 80006000 <_trampoline>
    800023f2:	00004697          	auipc	a3,0x4
    800023f6:	c0e68693          	add	a3,a3,-1010 # 80006000 <_trampoline>
    800023fa:	8f95                	sub	a5,a5,a3
    800023fc:	177d                	add	a4,a4,-1 # 3ffffff <_entry-0x7c000001>
    800023fe:	0732                	sll	a4,a4,0xc
    80002400:	97ba                	add	a5,a5,a4
    80002402:	10579073          	csrw	stvec,a5
    80002406:	6d3c                	ld	a5,88(a0)
    80002408:	18002773          	csrr	a4,satp
    8000240c:	e398                	sd	a4,0(a5)
    8000240e:	6d38                	ld	a4,88(a0)
    80002410:	613c                	ld	a5,64(a0)
    80002412:	6685                	lui	a3,0x1
    80002414:	97b6                	add	a5,a5,a3
    80002416:	e71c                	sd	a5,8(a4)
    80002418:	6d3c                	ld	a5,88(a0)
    8000241a:	00000717          	auipc	a4,0x0
    8000241e:	0f470713          	add	a4,a4,244 # 8000250e <usertrap>
    80002422:	eb98                	sd	a4,16(a5)
    80002424:	6d3c                	ld	a5,88(a0)
    80002426:	8712                	mv	a4,tp
    80002428:	f398                	sd	a4,32(a5)
    8000242a:	100027f3          	csrr	a5,sstatus
    8000242e:	eff7f793          	and	a5,a5,-257
    80002432:	0207e793          	or	a5,a5,32
    80002436:	10079073          	csrw	sstatus,a5
    8000243a:	6d3c                	ld	a5,88(a0)
    8000243c:	6f9c                	ld	a5,24(a5)
    8000243e:	14179073          	csrw	sepc,a5
    80002442:	60a2                	ld	ra,8(sp)
    80002444:	6402                	ld	s0,0(sp)
    80002446:	0141                	add	sp,sp,16
    80002448:	8082                	ret

000000008000244a <clockintr>:
    8000244a:	1101                	add	sp,sp,-32
    8000244c:	ec06                	sd	ra,24(sp)
    8000244e:	e822                	sd	s0,16(sp)
    80002450:	e426                	sd	s1,8(sp)
    80002452:	1000                	add	s0,sp,32
    80002454:	c0cff0ef          	jal	80001860 <cpuid>
    80002458:	cd19                	beqz	a0,80002476 <clockintr+0x2c>
    8000245a:	c01027f3          	rdtime	a5
    8000245e:	000f4737          	lui	a4,0xf4
    80002462:	24070713          	add	a4,a4,576 # f4240 <_entry-0x7ff0bdc0>
    80002466:	97ba                	add	a5,a5,a4
    80002468:	14d79073          	csrw	stimecmp,a5
    8000246c:	60e2                	ld	ra,24(sp)
    8000246e:	6442                	ld	s0,16(sp)
    80002470:	64a2                	ld	s1,8(sp)
    80002472:	6105                	add	sp,sp,32
    80002474:	8082                	ret
    80002476:	00013497          	auipc	s1,0x13
    8000247a:	31248493          	add	s1,s1,786 # 80015788 <tickslock>
    8000247e:	8526                	mv	a0,s1
    80002480:	f36fe0ef          	jal	80000bb6 <acquire>
    80002484:	00005517          	auipc	a0,0x5
    80002488:	3d450513          	add	a0,a0,980 # 80007858 <ticks>
    8000248c:	411c                	lw	a5,0(a0)
    8000248e:	2785                	addw	a5,a5,1
    80002490:	c11c                	sw	a5,0(a0)
    80002492:	a4fff0ef          	jal	80001ee0 <wakeup>
    80002496:	8526                	mv	a0,s1
    80002498:	fb6fe0ef          	jal	80000c4e <release>
    8000249c:	bf7d                	j	8000245a <clockintr+0x10>

000000008000249e <devintr>:
    8000249e:	1101                	add	sp,sp,-32
    800024a0:	ec06                	sd	ra,24(sp)
    800024a2:	e822                	sd	s0,16(sp)
    800024a4:	e426                	sd	s1,8(sp)
    800024a6:	1000                	add	s0,sp,32
    800024a8:	14202773          	csrr	a4,scause
    800024ac:	57fd                	li	a5,-1
    800024ae:	17fe                	sll	a5,a5,0x3f
    800024b0:	07a5                	add	a5,a5,9
    800024b2:	00f70d63          	beq	a4,a5,800024cc <devintr+0x2e>
    800024b6:	57fd                	li	a5,-1
    800024b8:	17fe                	sll	a5,a5,0x3f
    800024ba:	0795                	add	a5,a5,5
    800024bc:	4501                	li	a0,0
    800024be:	04f70463          	beq	a4,a5,80002506 <devintr+0x68>
    800024c2:	60e2                	ld	ra,24(sp)
    800024c4:	6442                	ld	s0,16(sp)
    800024c6:	64a2                	ld	s1,8(sp)
    800024c8:	6105                	add	sp,sp,32
    800024ca:	8082                	ret
    800024cc:	6bd020ef          	jal	80005388 <plic_claim>
    800024d0:	84aa                	mv	s1,a0
    800024d2:	47a9                	li	a5,10
    800024d4:	02f50363          	beq	a0,a5,800024fa <devintr+0x5c>
    800024d8:	4785                	li	a5,1
    800024da:	02f50363          	beq	a0,a5,80002500 <devintr+0x62>
    800024de:	4505                	li	a0,1
    800024e0:	d0ed                	beqz	s1,800024c2 <devintr+0x24>
    800024e2:	85a6                	mv	a1,s1
    800024e4:	00005517          	auipc	a0,0x5
    800024e8:	db450513          	add	a0,a0,-588 # 80007298 <states.1753+0x128>
    800024ec:	808fe0ef          	jal	800004f4 <printf>
    800024f0:	8526                	mv	a0,s1
    800024f2:	6b7020ef          	jal	800053a8 <plic_complete>
    800024f6:	4505                	li	a0,1
    800024f8:	b7e9                	j	800024c2 <devintr+0x24>
    800024fa:	ca2fe0ef          	jal	8000099c <uartintr>
    800024fe:	bfcd                	j	800024f0 <devintr+0x52>
    80002500:	332030ef          	jal	80005832 <virtio_disk_intr>
    80002504:	b7f5                	j	800024f0 <devintr+0x52>
    80002506:	f45ff0ef          	jal	8000244a <clockintr>
    8000250a:	4509                	li	a0,2
    8000250c:	bf5d                	j	800024c2 <devintr+0x24>

000000008000250e <usertrap>:
    8000250e:	1101                	add	sp,sp,-32
    80002510:	ec06                	sd	ra,24(sp)
    80002512:	e822                	sd	s0,16(sp)
    80002514:	e426                	sd	s1,8(sp)
    80002516:	e04a                	sd	s2,0(sp)
    80002518:	1000                	add	s0,sp,32
    8000251a:	100027f3          	csrr	a5,sstatus
    8000251e:	1007f793          	and	a5,a5,256
    80002522:	eba5                	bnez	a5,80002592 <usertrap+0x84>
    80002524:	00003797          	auipc	a5,0x3
    80002528:	dbc78793          	add	a5,a5,-580 # 800052e0 <kernelvec>
    8000252c:	10579073          	csrw	stvec,a5
    80002530:	b5cff0ef          	jal	8000188c <myproc>
    80002534:	84aa                	mv	s1,a0
    80002536:	6d3c                	ld	a5,88(a0)
    80002538:	14102773          	csrr	a4,sepc
    8000253c:	ef98                	sd	a4,24(a5)
    8000253e:	14202773          	csrr	a4,scause
    80002542:	47a1                	li	a5,8
    80002544:	04f70d63          	beq	a4,a5,8000259e <usertrap+0x90>
    80002548:	f57ff0ef          	jal	8000249e <devintr>
    8000254c:	892a                	mv	s2,a0
    8000254e:	e945                	bnez	a0,800025fe <usertrap+0xf0>
    80002550:	14202773          	csrr	a4,scause
    80002554:	47bd                	li	a5,15
    80002556:	08f70863          	beq	a4,a5,800025e6 <usertrap+0xd8>
    8000255a:	14202773          	csrr	a4,scause
    8000255e:	47b5                	li	a5,13
    80002560:	08f70363          	beq	a4,a5,800025e6 <usertrap+0xd8>
    80002564:	142025f3          	csrr	a1,scause
    80002568:	5890                	lw	a2,48(s1)
    8000256a:	00005517          	auipc	a0,0x5
    8000256e:	d6e50513          	add	a0,a0,-658 # 800072d8 <states.1753+0x168>
    80002572:	f83fd0ef          	jal	800004f4 <printf>
    80002576:	141025f3          	csrr	a1,sepc
    8000257a:	14302673          	csrr	a2,stval
    8000257e:	00005517          	auipc	a0,0x5
    80002582:	d8a50513          	add	a0,a0,-630 # 80007308 <states.1753+0x198>
    80002586:	f6ffd0ef          	jal	800004f4 <printf>
    8000258a:	8526                	mv	a0,s1
    8000258c:	b1fff0ef          	jal	800020aa <setkilled>
    80002590:	a035                	j	800025bc <usertrap+0xae>
    80002592:	00005517          	auipc	a0,0x5
    80002596:	d2650513          	add	a0,a0,-730 # 800072b8 <states.1753+0x148>
    8000259a:	a28fe0ef          	jal	800007c2 <panic>
    8000259e:	b31ff0ef          	jal	800020ce <killed>
    800025a2:	ed15                	bnez	a0,800025de <usertrap+0xd0>
    800025a4:	6cb8                	ld	a4,88(s1)
    800025a6:	6f1c                	ld	a5,24(a4)
    800025a8:	0791                	add	a5,a5,4
    800025aa:	ef1c                	sd	a5,24(a4)
    800025ac:	100027f3          	csrr	a5,sstatus
    800025b0:	0027e793          	or	a5,a5,2
    800025b4:	10079073          	csrw	sstatus,a5
    800025b8:	246000ef          	jal	800027fe <syscall>
    800025bc:	8526                	mv	a0,s1
    800025be:	b11ff0ef          	jal	800020ce <killed>
    800025c2:	e139                	bnez	a0,80002608 <usertrap+0xfa>
    800025c4:	e0dff0ef          	jal	800023d0 <prepare_return>
    800025c8:	68a8                	ld	a0,80(s1)
    800025ca:	8131                	srl	a0,a0,0xc
    800025cc:	57fd                	li	a5,-1
    800025ce:	17fe                	sll	a5,a5,0x3f
    800025d0:	8d5d                	or	a0,a0,a5
    800025d2:	60e2                	ld	ra,24(sp)
    800025d4:	6442                	ld	s0,16(sp)
    800025d6:	64a2                	ld	s1,8(sp)
    800025d8:	6902                	ld	s2,0(sp)
    800025da:	6105                	add	sp,sp,32
    800025dc:	8082                	ret
    800025de:	557d                	li	a0,-1
    800025e0:	9c1ff0ef          	jal	80001fa0 <kexit>
    800025e4:	b7c1                	j	800025a4 <usertrap+0x96>
    800025e6:	143025f3          	csrr	a1,stval
    800025ea:	14202673          	csrr	a2,scause
    800025ee:	164d                	add	a2,a2,-13 # ff3 <_entry-0x7ffff00d>
    800025f0:	00163613          	seqz	a2,a2
    800025f4:	68a8                	ld	a0,80(s1)
    800025f6:	f6ffe0ef          	jal	80001564 <vmfault>
    800025fa:	f169                	bnez	a0,800025bc <usertrap+0xae>
    800025fc:	b7a5                	j	80002564 <usertrap+0x56>
    800025fe:	8526                	mv	a0,s1
    80002600:	acfff0ef          	jal	800020ce <killed>
    80002604:	c511                	beqz	a0,80002610 <usertrap+0x102>
    80002606:	a011                	j	8000260a <usertrap+0xfc>
    80002608:	4901                	li	s2,0
    8000260a:	557d                	li	a0,-1
    8000260c:	995ff0ef          	jal	80001fa0 <kexit>
    80002610:	4789                	li	a5,2
    80002612:	faf919e3          	bne	s2,a5,800025c4 <usertrap+0xb6>
    80002616:	853ff0ef          	jal	80001e68 <yield>
    8000261a:	b76d                	j	800025c4 <usertrap+0xb6>

000000008000261c <kerneltrap>:
    8000261c:	7179                	add	sp,sp,-48
    8000261e:	f406                	sd	ra,40(sp)
    80002620:	f022                	sd	s0,32(sp)
    80002622:	ec26                	sd	s1,24(sp)
    80002624:	e84a                	sd	s2,16(sp)
    80002626:	e44e                	sd	s3,8(sp)
    80002628:	1800                	add	s0,sp,48
    8000262a:	14102973          	csrr	s2,sepc
    8000262e:	100024f3          	csrr	s1,sstatus
    80002632:	142029f3          	csrr	s3,scause
    80002636:	1004f793          	and	a5,s1,256
    8000263a:	c795                	beqz	a5,80002666 <kerneltrap+0x4a>
    8000263c:	100027f3          	csrr	a5,sstatus
    80002640:	8b89                	and	a5,a5,2
    80002642:	eb85                	bnez	a5,80002672 <kerneltrap+0x56>
    80002644:	e5bff0ef          	jal	8000249e <devintr>
    80002648:	c91d                	beqz	a0,8000267e <kerneltrap+0x62>
    8000264a:	4789                	li	a5,2
    8000264c:	04f50a63          	beq	a0,a5,800026a0 <kerneltrap+0x84>
    80002650:	14191073          	csrw	sepc,s2
    80002654:	10049073          	csrw	sstatus,s1
    80002658:	70a2                	ld	ra,40(sp)
    8000265a:	7402                	ld	s0,32(sp)
    8000265c:	64e2                	ld	s1,24(sp)
    8000265e:	6942                	ld	s2,16(sp)
    80002660:	69a2                	ld	s3,8(sp)
    80002662:	6145                	add	sp,sp,48
    80002664:	8082                	ret
    80002666:	00005517          	auipc	a0,0x5
    8000266a:	cca50513          	add	a0,a0,-822 # 80007330 <states.1753+0x1c0>
    8000266e:	954fe0ef          	jal	800007c2 <panic>
    80002672:	00005517          	auipc	a0,0x5
    80002676:	ce650513          	add	a0,a0,-794 # 80007358 <states.1753+0x1e8>
    8000267a:	948fe0ef          	jal	800007c2 <panic>
    8000267e:	14102673          	csrr	a2,sepc
    80002682:	143026f3          	csrr	a3,stval
    80002686:	85ce                	mv	a1,s3
    80002688:	00005517          	auipc	a0,0x5
    8000268c:	cf050513          	add	a0,a0,-784 # 80007378 <states.1753+0x208>
    80002690:	e65fd0ef          	jal	800004f4 <printf>
    80002694:	00005517          	auipc	a0,0x5
    80002698:	d0c50513          	add	a0,a0,-756 # 800073a0 <states.1753+0x230>
    8000269c:	926fe0ef          	jal	800007c2 <panic>
    800026a0:	9ecff0ef          	jal	8000188c <myproc>
    800026a4:	d555                	beqz	a0,80002650 <kerneltrap+0x34>
    800026a6:	fc2ff0ef          	jal	80001e68 <yield>
    800026aa:	b75d                	j	80002650 <kerneltrap+0x34>

00000000800026ac <argraw>:
    800026ac:	1101                	add	sp,sp,-32
    800026ae:	ec06                	sd	ra,24(sp)
    800026b0:	e822                	sd	s0,16(sp)
    800026b2:	e426                	sd	s1,8(sp)
    800026b4:	1000                	add	s0,sp,32
    800026b6:	84aa                	mv	s1,a0
    800026b8:	9d4ff0ef          	jal	8000188c <myproc>
    800026bc:	4795                	li	a5,5
    800026be:	0497e163          	bltu	a5,s1,80002700 <argraw+0x54>
    800026c2:	048a                	sll	s1,s1,0x2
    800026c4:	00005717          	auipc	a4,0x5
    800026c8:	cec70713          	add	a4,a4,-788 # 800073b0 <states.1753+0x240>
    800026cc:	94ba                	add	s1,s1,a4
    800026ce:	409c                	lw	a5,0(s1)
    800026d0:	97ba                	add	a5,a5,a4
    800026d2:	8782                	jr	a5
    800026d4:	6d3c                	ld	a5,88(a0)
    800026d6:	7ba8                	ld	a0,112(a5)
    800026d8:	60e2                	ld	ra,24(sp)
    800026da:	6442                	ld	s0,16(sp)
    800026dc:	64a2                	ld	s1,8(sp)
    800026de:	6105                	add	sp,sp,32
    800026e0:	8082                	ret
    800026e2:	6d3c                	ld	a5,88(a0)
    800026e4:	7fa8                	ld	a0,120(a5)
    800026e6:	bfcd                	j	800026d8 <argraw+0x2c>
    800026e8:	6d3c                	ld	a5,88(a0)
    800026ea:	63c8                	ld	a0,128(a5)
    800026ec:	b7f5                	j	800026d8 <argraw+0x2c>
    800026ee:	6d3c                	ld	a5,88(a0)
    800026f0:	67c8                	ld	a0,136(a5)
    800026f2:	b7dd                	j	800026d8 <argraw+0x2c>
    800026f4:	6d3c                	ld	a5,88(a0)
    800026f6:	6bc8                	ld	a0,144(a5)
    800026f8:	b7c5                	j	800026d8 <argraw+0x2c>
    800026fa:	6d3c                	ld	a5,88(a0)
    800026fc:	6fc8                	ld	a0,152(a5)
    800026fe:	bfe9                	j	800026d8 <argraw+0x2c>
    80002700:	00005517          	auipc	a0,0x5
    80002704:	d7850513          	add	a0,a0,-648 # 80007478 <syscalls+0xb0>
    80002708:	8bafe0ef          	jal	800007c2 <panic>

000000008000270c <fetchaddr>:
    8000270c:	1101                	add	sp,sp,-32
    8000270e:	ec06                	sd	ra,24(sp)
    80002710:	e822                	sd	s0,16(sp)
    80002712:	e426                	sd	s1,8(sp)
    80002714:	e04a                	sd	s2,0(sp)
    80002716:	1000                	add	s0,sp,32
    80002718:	84aa                	mv	s1,a0
    8000271a:	892e                	mv	s2,a1
    8000271c:	970ff0ef          	jal	8000188c <myproc>
    80002720:	653c                	ld	a5,72(a0)
    80002722:	02f4f663          	bgeu	s1,a5,8000274e <fetchaddr+0x42>
    80002726:	00848713          	add	a4,s1,8
    8000272a:	02e7e463          	bltu	a5,a4,80002752 <fetchaddr+0x46>
    8000272e:	46a1                	li	a3,8
    80002730:	8626                	mv	a2,s1
    80002732:	85ca                	mv	a1,s2
    80002734:	6928                	ld	a0,80(a0)
    80002736:	f67fe0ef          	jal	8000169c <copyin>
    8000273a:	00a03533          	snez	a0,a0
    8000273e:	40a00533          	neg	a0,a0
    80002742:	60e2                	ld	ra,24(sp)
    80002744:	6442                	ld	s0,16(sp)
    80002746:	64a2                	ld	s1,8(sp)
    80002748:	6902                	ld	s2,0(sp)
    8000274a:	6105                	add	sp,sp,32
    8000274c:	8082                	ret
    8000274e:	557d                	li	a0,-1
    80002750:	bfcd                	j	80002742 <fetchaddr+0x36>
    80002752:	557d                	li	a0,-1
    80002754:	b7fd                	j	80002742 <fetchaddr+0x36>

0000000080002756 <fetchstr>:
    80002756:	7179                	add	sp,sp,-48
    80002758:	f406                	sd	ra,40(sp)
    8000275a:	f022                	sd	s0,32(sp)
    8000275c:	ec26                	sd	s1,24(sp)
    8000275e:	e84a                	sd	s2,16(sp)
    80002760:	e44e                	sd	s3,8(sp)
    80002762:	1800                	add	s0,sp,48
    80002764:	892a                	mv	s2,a0
    80002766:	84ae                	mv	s1,a1
    80002768:	89b2                	mv	s3,a2
    8000276a:	922ff0ef          	jal	8000188c <myproc>
    8000276e:	86ce                	mv	a3,s3
    80002770:	864a                	mv	a2,s2
    80002772:	85a6                	mv	a1,s1
    80002774:	6928                	ld	a0,80(a0)
    80002776:	d0dfe0ef          	jal	80001482 <copyinstr>
    8000277a:	00054c63          	bltz	a0,80002792 <fetchstr+0x3c>
    8000277e:	8526                	mv	a0,s1
    80002780:	eacfe0ef          	jal	80000e2c <strlen>
    80002784:	70a2                	ld	ra,40(sp)
    80002786:	7402                	ld	s0,32(sp)
    80002788:	64e2                	ld	s1,24(sp)
    8000278a:	6942                	ld	s2,16(sp)
    8000278c:	69a2                	ld	s3,8(sp)
    8000278e:	6145                	add	sp,sp,48
    80002790:	8082                	ret
    80002792:	557d                	li	a0,-1
    80002794:	bfc5                	j	80002784 <fetchstr+0x2e>

0000000080002796 <argint>:
    80002796:	1101                	add	sp,sp,-32
    80002798:	ec06                	sd	ra,24(sp)
    8000279a:	e822                	sd	s0,16(sp)
    8000279c:	e426                	sd	s1,8(sp)
    8000279e:	1000                	add	s0,sp,32
    800027a0:	84ae                	mv	s1,a1
    800027a2:	f0bff0ef          	jal	800026ac <argraw>
    800027a6:	c088                	sw	a0,0(s1)
    800027a8:	60e2                	ld	ra,24(sp)
    800027aa:	6442                	ld	s0,16(sp)
    800027ac:	64a2                	ld	s1,8(sp)
    800027ae:	6105                	add	sp,sp,32
    800027b0:	8082                	ret

00000000800027b2 <argaddr>:
    800027b2:	1101                	add	sp,sp,-32
    800027b4:	ec06                	sd	ra,24(sp)
    800027b6:	e822                	sd	s0,16(sp)
    800027b8:	e426                	sd	s1,8(sp)
    800027ba:	1000                	add	s0,sp,32
    800027bc:	84ae                	mv	s1,a1
    800027be:	eefff0ef          	jal	800026ac <argraw>
    800027c2:	e088                	sd	a0,0(s1)
    800027c4:	60e2                	ld	ra,24(sp)
    800027c6:	6442                	ld	s0,16(sp)
    800027c8:	64a2                	ld	s1,8(sp)
    800027ca:	6105                	add	sp,sp,32
    800027cc:	8082                	ret

00000000800027ce <argstr>:
    800027ce:	7179                	add	sp,sp,-48
    800027d0:	f406                	sd	ra,40(sp)
    800027d2:	f022                	sd	s0,32(sp)
    800027d4:	ec26                	sd	s1,24(sp)
    800027d6:	e84a                	sd	s2,16(sp)
    800027d8:	1800                	add	s0,sp,48
    800027da:	84ae                	mv	s1,a1
    800027dc:	8932                	mv	s2,a2
    800027de:	fd840593          	add	a1,s0,-40
    800027e2:	fd1ff0ef          	jal	800027b2 <argaddr>
    800027e6:	864a                	mv	a2,s2
    800027e8:	85a6                	mv	a1,s1
    800027ea:	fd843503          	ld	a0,-40(s0)
    800027ee:	f69ff0ef          	jal	80002756 <fetchstr>
    800027f2:	70a2                	ld	ra,40(sp)
    800027f4:	7402                	ld	s0,32(sp)
    800027f6:	64e2                	ld	s1,24(sp)
    800027f8:	6942                	ld	s2,16(sp)
    800027fa:	6145                	add	sp,sp,48
    800027fc:	8082                	ret

00000000800027fe <syscall>:
    800027fe:	1101                	add	sp,sp,-32
    80002800:	ec06                	sd	ra,24(sp)
    80002802:	e822                	sd	s0,16(sp)
    80002804:	e426                	sd	s1,8(sp)
    80002806:	e04a                	sd	s2,0(sp)
    80002808:	1000                	add	s0,sp,32
    8000280a:	882ff0ef          	jal	8000188c <myproc>
    8000280e:	84aa                	mv	s1,a0
    80002810:	05853903          	ld	s2,88(a0)
    80002814:	0a893783          	ld	a5,168(s2)
    80002818:	0007869b          	sext.w	a3,a5
    8000281c:	37fd                	addw	a5,a5,-1
    8000281e:	4751                	li	a4,20
    80002820:	00f76f63          	bltu	a4,a5,8000283e <syscall+0x40>
    80002824:	00369713          	sll	a4,a3,0x3
    80002828:	00005797          	auipc	a5,0x5
    8000282c:	ba078793          	add	a5,a5,-1120 # 800073c8 <syscalls>
    80002830:	97ba                	add	a5,a5,a4
    80002832:	639c                	ld	a5,0(a5)
    80002834:	c789                	beqz	a5,8000283e <syscall+0x40>
    80002836:	9782                	jalr	a5
    80002838:	06a93823          	sd	a0,112(s2)
    8000283c:	a829                	j	80002856 <syscall+0x58>
    8000283e:	15848613          	add	a2,s1,344
    80002842:	588c                	lw	a1,48(s1)
    80002844:	00005517          	auipc	a0,0x5
    80002848:	c3c50513          	add	a0,a0,-964 # 80007480 <syscalls+0xb8>
    8000284c:	ca9fd0ef          	jal	800004f4 <printf>
    80002850:	6cbc                	ld	a5,88(s1)
    80002852:	577d                	li	a4,-1
    80002854:	fbb8                	sd	a4,112(a5)
    80002856:	60e2                	ld	ra,24(sp)
    80002858:	6442                	ld	s0,16(sp)
    8000285a:	64a2                	ld	s1,8(sp)
    8000285c:	6902                	ld	s2,0(sp)
    8000285e:	6105                	add	sp,sp,32
    80002860:	8082                	ret

0000000080002862 <sys_exit>:
    80002862:	1101                	add	sp,sp,-32
    80002864:	ec06                	sd	ra,24(sp)
    80002866:	e822                	sd	s0,16(sp)
    80002868:	1000                	add	s0,sp,32
    8000286a:	fec40593          	add	a1,s0,-20
    8000286e:	4501                	li	a0,0
    80002870:	f27ff0ef          	jal	80002796 <argint>
    80002874:	fec42503          	lw	a0,-20(s0)
    80002878:	f28ff0ef          	jal	80001fa0 <kexit>
    8000287c:	4501                	li	a0,0
    8000287e:	60e2                	ld	ra,24(sp)
    80002880:	6442                	ld	s0,16(sp)
    80002882:	6105                	add	sp,sp,32
    80002884:	8082                	ret

0000000080002886 <sys_getpid>:
    80002886:	1141                	add	sp,sp,-16
    80002888:	e406                	sd	ra,8(sp)
    8000288a:	e022                	sd	s0,0(sp)
    8000288c:	0800                	add	s0,sp,16
    8000288e:	ffffe0ef          	jal	8000188c <myproc>
    80002892:	5908                	lw	a0,48(a0)
    80002894:	60a2                	ld	ra,8(sp)
    80002896:	6402                	ld	s0,0(sp)
    80002898:	0141                	add	sp,sp,16
    8000289a:	8082                	ret

000000008000289c <sys_fork>:
    8000289c:	1141                	add	sp,sp,-16
    8000289e:	e406                	sd	ra,8(sp)
    800028a0:	e022                	sd	s0,0(sp)
    800028a2:	0800                	add	s0,sp,16
    800028a4:	b4eff0ef          	jal	80001bf2 <kfork>
    800028a8:	60a2                	ld	ra,8(sp)
    800028aa:	6402                	ld	s0,0(sp)
    800028ac:	0141                	add	sp,sp,16
    800028ae:	8082                	ret

00000000800028b0 <sys_wait>:
    800028b0:	1101                	add	sp,sp,-32
    800028b2:	ec06                	sd	ra,24(sp)
    800028b4:	e822                	sd	s0,16(sp)
    800028b6:	1000                	add	s0,sp,32
    800028b8:	fe840593          	add	a1,s0,-24
    800028bc:	4501                	li	a0,0
    800028be:	ef5ff0ef          	jal	800027b2 <argaddr>
    800028c2:	fe843503          	ld	a0,-24(s0)
    800028c6:	833ff0ef          	jal	800020f8 <kwait>
    800028ca:	60e2                	ld	ra,24(sp)
    800028cc:	6442                	ld	s0,16(sp)
    800028ce:	6105                	add	sp,sp,32
    800028d0:	8082                	ret

00000000800028d2 <sys_sbrk>:
    800028d2:	7179                	add	sp,sp,-48
    800028d4:	f406                	sd	ra,40(sp)
    800028d6:	f022                	sd	s0,32(sp)
    800028d8:	ec26                	sd	s1,24(sp)
    800028da:	1800                	add	s0,sp,48
    800028dc:	fd840593          	add	a1,s0,-40
    800028e0:	4501                	li	a0,0
    800028e2:	eb5ff0ef          	jal	80002796 <argint>
    800028e6:	fdc40593          	add	a1,s0,-36
    800028ea:	4505                	li	a0,1
    800028ec:	eabff0ef          	jal	80002796 <argint>
    800028f0:	f9dfe0ef          	jal	8000188c <myproc>
    800028f4:	6524                	ld	s1,72(a0)
    800028f6:	fdc42703          	lw	a4,-36(s0)
    800028fa:	4785                	li	a5,1
    800028fc:	02f70763          	beq	a4,a5,8000292a <sys_sbrk+0x58>
    80002900:	fd842783          	lw	a5,-40(s0)
    80002904:	0207c363          	bltz	a5,8000292a <sys_sbrk+0x58>
    80002908:	97a6                	add	a5,a5,s1
    8000290a:	0297ee63          	bltu	a5,s1,80002946 <sys_sbrk+0x74>
    8000290e:	02000737          	lui	a4,0x2000
    80002912:	177d                	add	a4,a4,-1 # 1ffffff <_entry-0x7e000001>
    80002914:	0736                	sll	a4,a4,0xd
    80002916:	02f76a63          	bltu	a4,a5,8000294a <sys_sbrk+0x78>
    8000291a:	f73fe0ef          	jal	8000188c <myproc>
    8000291e:	fd842703          	lw	a4,-40(s0)
    80002922:	653c                	ld	a5,72(a0)
    80002924:	97ba                	add	a5,a5,a4
    80002926:	e53c                	sd	a5,72(a0)
    80002928:	a039                	j	80002936 <sys_sbrk+0x64>
    8000292a:	fd842503          	lw	a0,-40(s0)
    8000292e:	a62ff0ef          	jal	80001b90 <growproc>
    80002932:	00054863          	bltz	a0,80002942 <sys_sbrk+0x70>
    80002936:	8526                	mv	a0,s1
    80002938:	70a2                	ld	ra,40(sp)
    8000293a:	7402                	ld	s0,32(sp)
    8000293c:	64e2                	ld	s1,24(sp)
    8000293e:	6145                	add	sp,sp,48
    80002940:	8082                	ret
    80002942:	54fd                	li	s1,-1
    80002944:	bfcd                	j	80002936 <sys_sbrk+0x64>
    80002946:	54fd                	li	s1,-1
    80002948:	b7fd                	j	80002936 <sys_sbrk+0x64>
    8000294a:	54fd                	li	s1,-1
    8000294c:	b7ed                	j	80002936 <sys_sbrk+0x64>

000000008000294e <sys_pause>:
    8000294e:	7139                	add	sp,sp,-64
    80002950:	fc06                	sd	ra,56(sp)
    80002952:	f822                	sd	s0,48(sp)
    80002954:	f426                	sd	s1,40(sp)
    80002956:	f04a                	sd	s2,32(sp)
    80002958:	ec4e                	sd	s3,24(sp)
    8000295a:	0080                	add	s0,sp,64
    8000295c:	fcc40593          	add	a1,s0,-52
    80002960:	4501                	li	a0,0
    80002962:	e35ff0ef          	jal	80002796 <argint>
    80002966:	fcc42783          	lw	a5,-52(s0)
    8000296a:	0607c763          	bltz	a5,800029d8 <sys_pause+0x8a>
    8000296e:	00013517          	auipc	a0,0x13
    80002972:	e1a50513          	add	a0,a0,-486 # 80015788 <tickslock>
    80002976:	a40fe0ef          	jal	80000bb6 <acquire>
    8000297a:	00005797          	auipc	a5,0x5
    8000297e:	ede78793          	add	a5,a5,-290 # 80007858 <ticks>
    80002982:	0007a903          	lw	s2,0(a5)
    80002986:	fcc42783          	lw	a5,-52(s0)
    8000298a:	cb8d                	beqz	a5,800029bc <sys_pause+0x6e>
    8000298c:	00013997          	auipc	s3,0x13
    80002990:	dfc98993          	add	s3,s3,-516 # 80015788 <tickslock>
    80002994:	00005497          	auipc	s1,0x5
    80002998:	ec448493          	add	s1,s1,-316 # 80007858 <ticks>
    8000299c:	ef1fe0ef          	jal	8000188c <myproc>
    800029a0:	f2eff0ef          	jal	800020ce <killed>
    800029a4:	ed0d                	bnez	a0,800029de <sys_pause+0x90>
    800029a6:	85ce                	mv	a1,s3
    800029a8:	8526                	mv	a0,s1
    800029aa:	ceaff0ef          	jal	80001e94 <sleep>
    800029ae:	409c                	lw	a5,0(s1)
    800029b0:	412787bb          	subw	a5,a5,s2
    800029b4:	fcc42703          	lw	a4,-52(s0)
    800029b8:	fee7e2e3          	bltu	a5,a4,8000299c <sys_pause+0x4e>
    800029bc:	00013517          	auipc	a0,0x13
    800029c0:	dcc50513          	add	a0,a0,-564 # 80015788 <tickslock>
    800029c4:	a8afe0ef          	jal	80000c4e <release>
    800029c8:	4501                	li	a0,0
    800029ca:	70e2                	ld	ra,56(sp)
    800029cc:	7442                	ld	s0,48(sp)
    800029ce:	74a2                	ld	s1,40(sp)
    800029d0:	7902                	ld	s2,32(sp)
    800029d2:	69e2                	ld	s3,24(sp)
    800029d4:	6121                	add	sp,sp,64
    800029d6:	8082                	ret
    800029d8:	fc042623          	sw	zero,-52(s0)
    800029dc:	bf49                	j	8000296e <sys_pause+0x20>
    800029de:	00013517          	auipc	a0,0x13
    800029e2:	daa50513          	add	a0,a0,-598 # 80015788 <tickslock>
    800029e6:	a68fe0ef          	jal	80000c4e <release>
    800029ea:	557d                	li	a0,-1
    800029ec:	bff9                	j	800029ca <sys_pause+0x7c>

00000000800029ee <sys_kill>:
    800029ee:	1101                	add	sp,sp,-32
    800029f0:	ec06                	sd	ra,24(sp)
    800029f2:	e822                	sd	s0,16(sp)
    800029f4:	1000                	add	s0,sp,32
    800029f6:	fec40593          	add	a1,s0,-20
    800029fa:	4501                	li	a0,0
    800029fc:	d9bff0ef          	jal	80002796 <argint>
    80002a00:	fec42503          	lw	a0,-20(s0)
    80002a04:	e40ff0ef          	jal	80002044 <kkill>
    80002a08:	60e2                	ld	ra,24(sp)
    80002a0a:	6442                	ld	s0,16(sp)
    80002a0c:	6105                	add	sp,sp,32
    80002a0e:	8082                	ret

0000000080002a10 <sys_uptime>:
    80002a10:	1101                	add	sp,sp,-32
    80002a12:	ec06                	sd	ra,24(sp)
    80002a14:	e822                	sd	s0,16(sp)
    80002a16:	e426                	sd	s1,8(sp)
    80002a18:	1000                	add	s0,sp,32
    80002a1a:	00013517          	auipc	a0,0x13
    80002a1e:	d6e50513          	add	a0,a0,-658 # 80015788 <tickslock>
    80002a22:	994fe0ef          	jal	80000bb6 <acquire>
    80002a26:	00005797          	auipc	a5,0x5
    80002a2a:	e3278793          	add	a5,a5,-462 # 80007858 <ticks>
    80002a2e:	4384                	lw	s1,0(a5)
    80002a30:	00013517          	auipc	a0,0x13
    80002a34:	d5850513          	add	a0,a0,-680 # 80015788 <tickslock>
    80002a38:	a16fe0ef          	jal	80000c4e <release>
    80002a3c:	02049513          	sll	a0,s1,0x20
    80002a40:	9101                	srl	a0,a0,0x20
    80002a42:	60e2                	ld	ra,24(sp)
    80002a44:	6442                	ld	s0,16(sp)
    80002a46:	64a2                	ld	s1,8(sp)
    80002a48:	6105                	add	sp,sp,32
    80002a4a:	8082                	ret

0000000080002a4c <binit>:
    80002a4c:	7179                	add	sp,sp,-48
    80002a4e:	f406                	sd	ra,40(sp)
    80002a50:	f022                	sd	s0,32(sp)
    80002a52:	ec26                	sd	s1,24(sp)
    80002a54:	e84a                	sd	s2,16(sp)
    80002a56:	e44e                	sd	s3,8(sp)
    80002a58:	e052                	sd	s4,0(sp)
    80002a5a:	1800                	add	s0,sp,48
    80002a5c:	00005597          	auipc	a1,0x5
    80002a60:	a4458593          	add	a1,a1,-1468 # 800074a0 <syscalls+0xd8>
    80002a64:	00013517          	auipc	a0,0x13
    80002a68:	d3c50513          	add	a0,a0,-708 # 800157a0 <bcache>
    80002a6c:	8cafe0ef          	jal	80000b36 <initlock>
    80002a70:	0001b797          	auipc	a5,0x1b
    80002a74:	d3078793          	add	a5,a5,-720 # 8001d7a0 <bcache+0x8000>
    80002a78:	0001b717          	auipc	a4,0x1b
    80002a7c:	f9070713          	add	a4,a4,-112 # 8001da08 <bcache+0x8268>
    80002a80:	2ae7b823          	sd	a4,688(a5)
    80002a84:	2ae7bc23          	sd	a4,696(a5)
    80002a88:	00013497          	auipc	s1,0x13
    80002a8c:	d3048493          	add	s1,s1,-720 # 800157b8 <bcache+0x18>
    80002a90:	893e                	mv	s2,a5
    80002a92:	89ba                	mv	s3,a4
    80002a94:	00005a17          	auipc	s4,0x5
    80002a98:	a14a0a13          	add	s4,s4,-1516 # 800074a8 <syscalls+0xe0>
    80002a9c:	2b893783          	ld	a5,696(s2)
    80002aa0:	e8bc                	sd	a5,80(s1)
    80002aa2:	0534b423          	sd	s3,72(s1)
    80002aa6:	85d2                	mv	a1,s4
    80002aa8:	01048513          	add	a0,s1,16
    80002aac:	360010ef          	jal	80003e0c <initsleeplock>
    80002ab0:	2b893783          	ld	a5,696(s2)
    80002ab4:	e7a4                	sd	s1,72(a5)
    80002ab6:	2a993c23          	sd	s1,696(s2)
    80002aba:	45848493          	add	s1,s1,1112
    80002abe:	fd349fe3          	bne	s1,s3,80002a9c <binit+0x50>
    80002ac2:	70a2                	ld	ra,40(sp)
    80002ac4:	7402                	ld	s0,32(sp)
    80002ac6:	64e2                	ld	s1,24(sp)
    80002ac8:	6942                	ld	s2,16(sp)
    80002aca:	69a2                	ld	s3,8(sp)
    80002acc:	6a02                	ld	s4,0(sp)
    80002ace:	6145                	add	sp,sp,48
    80002ad0:	8082                	ret

0000000080002ad2 <bread>:
    80002ad2:	7179                	add	sp,sp,-48
    80002ad4:	f406                	sd	ra,40(sp)
    80002ad6:	f022                	sd	s0,32(sp)
    80002ad8:	ec26                	sd	s1,24(sp)
    80002ada:	e84a                	sd	s2,16(sp)
    80002adc:	e44e                	sd	s3,8(sp)
    80002ade:	1800                	add	s0,sp,48
    80002ae0:	89aa                	mv	s3,a0
    80002ae2:	892e                	mv	s2,a1
    80002ae4:	00013517          	auipc	a0,0x13
    80002ae8:	cbc50513          	add	a0,a0,-836 # 800157a0 <bcache>
    80002aec:	8cafe0ef          	jal	80000bb6 <acquire>
    80002af0:	0001b797          	auipc	a5,0x1b
    80002af4:	cb078793          	add	a5,a5,-848 # 8001d7a0 <bcache+0x8000>
    80002af8:	2b87b483          	ld	s1,696(a5)
    80002afc:	0001b797          	auipc	a5,0x1b
    80002b00:	f0c78793          	add	a5,a5,-244 # 8001da08 <bcache+0x8268>
    80002b04:	02f48b63          	beq	s1,a5,80002b3a <bread+0x68>
    80002b08:	873e                	mv	a4,a5
    80002b0a:	a021                	j	80002b12 <bread+0x40>
    80002b0c:	68a4                	ld	s1,80(s1)
    80002b0e:	02e48663          	beq	s1,a4,80002b3a <bread+0x68>
    80002b12:	449c                	lw	a5,8(s1)
    80002b14:	ff379ce3          	bne	a5,s3,80002b0c <bread+0x3a>
    80002b18:	44dc                	lw	a5,12(s1)
    80002b1a:	ff2799e3          	bne	a5,s2,80002b0c <bread+0x3a>
    80002b1e:	40bc                	lw	a5,64(s1)
    80002b20:	2785                	addw	a5,a5,1
    80002b22:	c0bc                	sw	a5,64(s1)
    80002b24:	00013517          	auipc	a0,0x13
    80002b28:	c7c50513          	add	a0,a0,-900 # 800157a0 <bcache>
    80002b2c:	922fe0ef          	jal	80000c4e <release>
    80002b30:	01048513          	add	a0,s1,16
    80002b34:	30e010ef          	jal	80003e42 <acquiresleep>
    80002b38:	a891                	j	80002b8c <bread+0xba>
    80002b3a:	0001b797          	auipc	a5,0x1b
    80002b3e:	c6678793          	add	a5,a5,-922 # 8001d7a0 <bcache+0x8000>
    80002b42:	2b07b483          	ld	s1,688(a5)
    80002b46:	0001b797          	auipc	a5,0x1b
    80002b4a:	ec278793          	add	a5,a5,-318 # 8001da08 <bcache+0x8268>
    80002b4e:	04f48963          	beq	s1,a5,80002ba0 <bread+0xce>
    80002b52:	40bc                	lw	a5,64(s1)
    80002b54:	cb91                	beqz	a5,80002b68 <bread+0x96>
    80002b56:	0001b717          	auipc	a4,0x1b
    80002b5a:	eb270713          	add	a4,a4,-334 # 8001da08 <bcache+0x8268>
    80002b5e:	64a4                	ld	s1,72(s1)
    80002b60:	04e48063          	beq	s1,a4,80002ba0 <bread+0xce>
    80002b64:	40bc                	lw	a5,64(s1)
    80002b66:	ffe5                	bnez	a5,80002b5e <bread+0x8c>
    80002b68:	0134a423          	sw	s3,8(s1)
    80002b6c:	0124a623          	sw	s2,12(s1)
    80002b70:	0004a023          	sw	zero,0(s1)
    80002b74:	4785                	li	a5,1
    80002b76:	c0bc                	sw	a5,64(s1)
    80002b78:	00013517          	auipc	a0,0x13
    80002b7c:	c2850513          	add	a0,a0,-984 # 800157a0 <bcache>
    80002b80:	8cefe0ef          	jal	80000c4e <release>
    80002b84:	01048513          	add	a0,s1,16
    80002b88:	2ba010ef          	jal	80003e42 <acquiresleep>
    80002b8c:	409c                	lw	a5,0(s1)
    80002b8e:	cf99                	beqz	a5,80002bac <bread+0xda>
    80002b90:	8526                	mv	a0,s1
    80002b92:	70a2                	ld	ra,40(sp)
    80002b94:	7402                	ld	s0,32(sp)
    80002b96:	64e2                	ld	s1,24(sp)
    80002b98:	6942                	ld	s2,16(sp)
    80002b9a:	69a2                	ld	s3,8(sp)
    80002b9c:	6145                	add	sp,sp,48
    80002b9e:	8082                	ret
    80002ba0:	00005517          	auipc	a0,0x5
    80002ba4:	91050513          	add	a0,a0,-1776 # 800074b0 <syscalls+0xe8>
    80002ba8:	c1bfd0ef          	jal	800007c2 <panic>
    80002bac:	4581                	li	a1,0
    80002bae:	8526                	mv	a0,s1
    80002bb0:	249020ef          	jal	800055f8 <virtio_disk_rw>
    80002bb4:	4785                	li	a5,1
    80002bb6:	c09c                	sw	a5,0(s1)
    80002bb8:	bfe1                	j	80002b90 <bread+0xbe>

0000000080002bba <bwrite>:
    80002bba:	1101                	add	sp,sp,-32
    80002bbc:	ec06                	sd	ra,24(sp)
    80002bbe:	e822                	sd	s0,16(sp)
    80002bc0:	e426                	sd	s1,8(sp)
    80002bc2:	1000                	add	s0,sp,32
    80002bc4:	84aa                	mv	s1,a0
    80002bc6:	0541                	add	a0,a0,16
    80002bc8:	2f8010ef          	jal	80003ec0 <holdingsleep>
    80002bcc:	c911                	beqz	a0,80002be0 <bwrite+0x26>
    80002bce:	4585                	li	a1,1
    80002bd0:	8526                	mv	a0,s1
    80002bd2:	227020ef          	jal	800055f8 <virtio_disk_rw>
    80002bd6:	60e2                	ld	ra,24(sp)
    80002bd8:	6442                	ld	s0,16(sp)
    80002bda:	64a2                	ld	s1,8(sp)
    80002bdc:	6105                	add	sp,sp,32
    80002bde:	8082                	ret
    80002be0:	00005517          	auipc	a0,0x5
    80002be4:	8e850513          	add	a0,a0,-1816 # 800074c8 <syscalls+0x100>
    80002be8:	bdbfd0ef          	jal	800007c2 <panic>

0000000080002bec <brelse>:
    80002bec:	1101                	add	sp,sp,-32
    80002bee:	ec06                	sd	ra,24(sp)
    80002bf0:	e822                	sd	s0,16(sp)
    80002bf2:	e426                	sd	s1,8(sp)
    80002bf4:	e04a                	sd	s2,0(sp)
    80002bf6:	1000                	add	s0,sp,32
    80002bf8:	84aa                	mv	s1,a0
    80002bfa:	01050913          	add	s2,a0,16
    80002bfe:	854a                	mv	a0,s2
    80002c00:	2c0010ef          	jal	80003ec0 <holdingsleep>
    80002c04:	c13d                	beqz	a0,80002c6a <brelse+0x7e>
    80002c06:	854a                	mv	a0,s2
    80002c08:	280010ef          	jal	80003e88 <releasesleep>
    80002c0c:	00013517          	auipc	a0,0x13
    80002c10:	b9450513          	add	a0,a0,-1132 # 800157a0 <bcache>
    80002c14:	fa3fd0ef          	jal	80000bb6 <acquire>
    80002c18:	40bc                	lw	a5,64(s1)
    80002c1a:	37fd                	addw	a5,a5,-1
    80002c1c:	0007871b          	sext.w	a4,a5
    80002c20:	c0bc                	sw	a5,64(s1)
    80002c22:	eb05                	bnez	a4,80002c52 <brelse+0x66>
    80002c24:	68bc                	ld	a5,80(s1)
    80002c26:	64b8                	ld	a4,72(s1)
    80002c28:	e7b8                	sd	a4,72(a5)
    80002c2a:	64bc                	ld	a5,72(s1)
    80002c2c:	68b8                	ld	a4,80(s1)
    80002c2e:	ebb8                	sd	a4,80(a5)
    80002c30:	0001b797          	auipc	a5,0x1b
    80002c34:	b7078793          	add	a5,a5,-1168 # 8001d7a0 <bcache+0x8000>
    80002c38:	2b87b703          	ld	a4,696(a5)
    80002c3c:	e8b8                	sd	a4,80(s1)
    80002c3e:	0001b717          	auipc	a4,0x1b
    80002c42:	dca70713          	add	a4,a4,-566 # 8001da08 <bcache+0x8268>
    80002c46:	e4b8                	sd	a4,72(s1)
    80002c48:	2b87b703          	ld	a4,696(a5)
    80002c4c:	e724                	sd	s1,72(a4)
    80002c4e:	2a97bc23          	sd	s1,696(a5)
    80002c52:	00013517          	auipc	a0,0x13
    80002c56:	b4e50513          	add	a0,a0,-1202 # 800157a0 <bcache>
    80002c5a:	ff5fd0ef          	jal	80000c4e <release>
    80002c5e:	60e2                	ld	ra,24(sp)
    80002c60:	6442                	ld	s0,16(sp)
    80002c62:	64a2                	ld	s1,8(sp)
    80002c64:	6902                	ld	s2,0(sp)
    80002c66:	6105                	add	sp,sp,32
    80002c68:	8082                	ret
    80002c6a:	00005517          	auipc	a0,0x5
    80002c6e:	86650513          	add	a0,a0,-1946 # 800074d0 <syscalls+0x108>
    80002c72:	b51fd0ef          	jal	800007c2 <panic>

0000000080002c76 <bpin>:
    80002c76:	1101                	add	sp,sp,-32
    80002c78:	ec06                	sd	ra,24(sp)
    80002c7a:	e822                	sd	s0,16(sp)
    80002c7c:	e426                	sd	s1,8(sp)
    80002c7e:	1000                	add	s0,sp,32
    80002c80:	84aa                	mv	s1,a0
    80002c82:	00013517          	auipc	a0,0x13
    80002c86:	b1e50513          	add	a0,a0,-1250 # 800157a0 <bcache>
    80002c8a:	f2dfd0ef          	jal	80000bb6 <acquire>
    80002c8e:	40bc                	lw	a5,64(s1)
    80002c90:	2785                	addw	a5,a5,1
    80002c92:	c0bc                	sw	a5,64(s1)
    80002c94:	00013517          	auipc	a0,0x13
    80002c98:	b0c50513          	add	a0,a0,-1268 # 800157a0 <bcache>
    80002c9c:	fb3fd0ef          	jal	80000c4e <release>
    80002ca0:	60e2                	ld	ra,24(sp)
    80002ca2:	6442                	ld	s0,16(sp)
    80002ca4:	64a2                	ld	s1,8(sp)
    80002ca6:	6105                	add	sp,sp,32
    80002ca8:	8082                	ret

0000000080002caa <bunpin>:
    80002caa:	1101                	add	sp,sp,-32
    80002cac:	ec06                	sd	ra,24(sp)
    80002cae:	e822                	sd	s0,16(sp)
    80002cb0:	e426                	sd	s1,8(sp)
    80002cb2:	1000                	add	s0,sp,32
    80002cb4:	84aa                	mv	s1,a0
    80002cb6:	00013517          	auipc	a0,0x13
    80002cba:	aea50513          	add	a0,a0,-1302 # 800157a0 <bcache>
    80002cbe:	ef9fd0ef          	jal	80000bb6 <acquire>
    80002cc2:	40bc                	lw	a5,64(s1)
    80002cc4:	37fd                	addw	a5,a5,-1
    80002cc6:	c0bc                	sw	a5,64(s1)
    80002cc8:	00013517          	auipc	a0,0x13
    80002ccc:	ad850513          	add	a0,a0,-1320 # 800157a0 <bcache>
    80002cd0:	f7ffd0ef          	jal	80000c4e <release>
    80002cd4:	60e2                	ld	ra,24(sp)
    80002cd6:	6442                	ld	s0,16(sp)
    80002cd8:	64a2                	ld	s1,8(sp)
    80002cda:	6105                	add	sp,sp,32
    80002cdc:	8082                	ret

0000000080002cde <bfree>:
    80002cde:	1101                	add	sp,sp,-32
    80002ce0:	ec06                	sd	ra,24(sp)
    80002ce2:	e822                	sd	s0,16(sp)
    80002ce4:	e426                	sd	s1,8(sp)
    80002ce6:	e04a                	sd	s2,0(sp)
    80002ce8:	1000                	add	s0,sp,32
    80002cea:	84ae                	mv	s1,a1
    80002cec:	00d5d59b          	srlw	a1,a1,0xd
    80002cf0:	0001b797          	auipc	a5,0x1b
    80002cf4:	17078793          	add	a5,a5,368 # 8001de60 <sb>
    80002cf8:	4fdc                	lw	a5,28(a5)
    80002cfa:	9dbd                	addw	a1,a1,a5
    80002cfc:	dd7ff0ef          	jal	80002ad2 <bread>
    80002d00:	2481                	sext.w	s1,s1
    80002d02:	0074f713          	and	a4,s1,7
    80002d06:	4785                	li	a5,1
    80002d08:	00e797bb          	sllw	a5,a5,a4
    80002d0c:	14ce                	sll	s1,s1,0x33
    80002d0e:	90d9                	srl	s1,s1,0x36
    80002d10:	00950733          	add	a4,a0,s1
    80002d14:	05874703          	lbu	a4,88(a4)
    80002d18:	00e7f6b3          	and	a3,a5,a4
    80002d1c:	c29d                	beqz	a3,80002d42 <bfree+0x64>
    80002d1e:	892a                	mv	s2,a0
    80002d20:	94aa                	add	s1,s1,a0
    80002d22:	fff7c793          	not	a5,a5
    80002d26:	8ff9                	and	a5,a5,a4
    80002d28:	04f48c23          	sb	a5,88(s1)
    80002d2c:	002010ef          	jal	80003d2e <log_write>
    80002d30:	854a                	mv	a0,s2
    80002d32:	ebbff0ef          	jal	80002bec <brelse>
    80002d36:	60e2                	ld	ra,24(sp)
    80002d38:	6442                	ld	s0,16(sp)
    80002d3a:	64a2                	ld	s1,8(sp)
    80002d3c:	6902                	ld	s2,0(sp)
    80002d3e:	6105                	add	sp,sp,32
    80002d40:	8082                	ret
    80002d42:	00004517          	auipc	a0,0x4
    80002d46:	79650513          	add	a0,a0,1942 # 800074d8 <syscalls+0x110>
    80002d4a:	a79fd0ef          	jal	800007c2 <panic>

0000000080002d4e <balloc>:
    80002d4e:	711d                	add	sp,sp,-96
    80002d50:	ec86                	sd	ra,88(sp)
    80002d52:	e8a2                	sd	s0,80(sp)
    80002d54:	e4a6                	sd	s1,72(sp)
    80002d56:	e0ca                	sd	s2,64(sp)
    80002d58:	fc4e                	sd	s3,56(sp)
    80002d5a:	f852                	sd	s4,48(sp)
    80002d5c:	f456                	sd	s5,40(sp)
    80002d5e:	f05a                	sd	s6,32(sp)
    80002d60:	ec5e                	sd	s7,24(sp)
    80002d62:	e862                	sd	s8,16(sp)
    80002d64:	e466                	sd	s9,8(sp)
    80002d66:	1080                	add	s0,sp,96
    80002d68:	0001b797          	auipc	a5,0x1b
    80002d6c:	0f878793          	add	a5,a5,248 # 8001de60 <sb>
    80002d70:	43dc                	lw	a5,4(a5)
    80002d72:	0e078e63          	beqz	a5,80002e6e <balloc+0x120>
    80002d76:	8baa                	mv	s7,a0
    80002d78:	4a81                	li	s5,0
    80002d7a:	0001bb17          	auipc	s6,0x1b
    80002d7e:	0e6b0b13          	add	s6,s6,230 # 8001de60 <sb>
    80002d82:	4c05                	li	s8,1
    80002d84:	4985                	li	s3,1
    80002d86:	6a09                	lui	s4,0x2
    80002d88:	6c89                	lui	s9,0x2
    80002d8a:	a88d                	j	80002dfc <balloc+0xae>
    80002d8c:	8942                	mv	s2,a6
    80002d8e:	4705                	li	a4,1
    80002d90:	4681                	li	a3,0
    80002d92:	96a6                	add	a3,a3,s1
    80002d94:	8f51                	or	a4,a4,a2
    80002d96:	04e68c23          	sb	a4,88(a3) # 1058 <_entry-0x7fffefa8>
    80002d9a:	8526                	mv	a0,s1
    80002d9c:	793000ef          	jal	80003d2e <log_write>
    80002da0:	8526                	mv	a0,s1
    80002da2:	e4bff0ef          	jal	80002bec <brelse>
    80002da6:	85ca                	mv	a1,s2
    80002da8:	855e                	mv	a0,s7
    80002daa:	d29ff0ef          	jal	80002ad2 <bread>
    80002dae:	84aa                	mv	s1,a0
    80002db0:	40000613          	li	a2,1024
    80002db4:	4581                	li	a1,0
    80002db6:	05850513          	add	a0,a0,88
    80002dba:	ed1fd0ef          	jal	80000c8a <memset>
    80002dbe:	8526                	mv	a0,s1
    80002dc0:	76f000ef          	jal	80003d2e <log_write>
    80002dc4:	8526                	mv	a0,s1
    80002dc6:	e27ff0ef          	jal	80002bec <brelse>
    80002dca:	854a                	mv	a0,s2
    80002dcc:	60e6                	ld	ra,88(sp)
    80002dce:	6446                	ld	s0,80(sp)
    80002dd0:	64a6                	ld	s1,72(sp)
    80002dd2:	6906                	ld	s2,64(sp)
    80002dd4:	79e2                	ld	s3,56(sp)
    80002dd6:	7a42                	ld	s4,48(sp)
    80002dd8:	7aa2                	ld	s5,40(sp)
    80002dda:	7b02                	ld	s6,32(sp)
    80002ddc:	6be2                	ld	s7,24(sp)
    80002dde:	6c42                	ld	s8,16(sp)
    80002de0:	6ca2                	ld	s9,8(sp)
    80002de2:	6125                	add	sp,sp,96
    80002de4:	8082                	ret
    80002de6:	8526                	mv	a0,s1
    80002de8:	e05ff0ef          	jal	80002bec <brelse>
    80002dec:	015c87bb          	addw	a5,s9,s5
    80002df0:	00078a9b          	sext.w	s5,a5
    80002df4:	004b2703          	lw	a4,4(s6)
    80002df8:	06eafb63          	bgeu	s5,a4,80002e6e <balloc+0x120>
    80002dfc:	41fad79b          	sraw	a5,s5,0x1f
    80002e00:	0137d79b          	srlw	a5,a5,0x13
    80002e04:	015787bb          	addw	a5,a5,s5
    80002e08:	40d7d79b          	sraw	a5,a5,0xd
    80002e0c:	01cb2583          	lw	a1,28(s6)
    80002e10:	9dbd                	addw	a1,a1,a5
    80002e12:	855e                	mv	a0,s7
    80002e14:	cbfff0ef          	jal	80002ad2 <bread>
    80002e18:	84aa                	mv	s1,a0
    80002e1a:	000a881b          	sext.w	a6,s5
    80002e1e:	004b2503          	lw	a0,4(s6)
    80002e22:	fca872e3          	bgeu	a6,a0,80002de6 <balloc+0x98>
    80002e26:	0584c603          	lbu	a2,88(s1)
    80002e2a:	00167793          	and	a5,a2,1
    80002e2e:	dfb9                	beqz	a5,80002d8c <balloc+0x3e>
    80002e30:	4105053b          	subw	a0,a0,a6
    80002e34:	87e2                	mv	a5,s8
    80002e36:	0107893b          	addw	s2,a5,a6
    80002e3a:	faa786e3          	beq	a5,a0,80002de6 <balloc+0x98>
    80002e3e:	41f7d71b          	sraw	a4,a5,0x1f
    80002e42:	01d7561b          	srlw	a2,a4,0x1d
    80002e46:	00f606bb          	addw	a3,a2,a5
    80002e4a:	0076f713          	and	a4,a3,7
    80002e4e:	9f11                	subw	a4,a4,a2
    80002e50:	00e9973b          	sllw	a4,s3,a4
    80002e54:	4036d69b          	sraw	a3,a3,0x3
    80002e58:	00d48633          	add	a2,s1,a3
    80002e5c:	05864603          	lbu	a2,88(a2)
    80002e60:	00c775b3          	and	a1,a4,a2
    80002e64:	d59d                	beqz	a1,80002d92 <balloc+0x44>
    80002e66:	2785                	addw	a5,a5,1
    80002e68:	fd4797e3          	bne	a5,s4,80002e36 <balloc+0xe8>
    80002e6c:	bfad                	j	80002de6 <balloc+0x98>
    80002e6e:	00004517          	auipc	a0,0x4
    80002e72:	68250513          	add	a0,a0,1666 # 800074f0 <syscalls+0x128>
    80002e76:	e7efd0ef          	jal	800004f4 <printf>
    80002e7a:	4901                	li	s2,0
    80002e7c:	b7b9                	j	80002dca <balloc+0x7c>

0000000080002e7e <bmap>:
    80002e7e:	7179                	add	sp,sp,-48
    80002e80:	f406                	sd	ra,40(sp)
    80002e82:	f022                	sd	s0,32(sp)
    80002e84:	ec26                	sd	s1,24(sp)
    80002e86:	e84a                	sd	s2,16(sp)
    80002e88:	e44e                	sd	s3,8(sp)
    80002e8a:	e052                	sd	s4,0(sp)
    80002e8c:	1800                	add	s0,sp,48
    80002e8e:	89aa                	mv	s3,a0
    80002e90:	47ad                	li	a5,11
    80002e92:	02b7e563          	bltu	a5,a1,80002ebc <bmap+0x3e>
    80002e96:	02059493          	sll	s1,a1,0x20
    80002e9a:	9081                	srl	s1,s1,0x20
    80002e9c:	048a                	sll	s1,s1,0x2
    80002e9e:	94aa                	add	s1,s1,a0
    80002ea0:	0504a903          	lw	s2,80(s1)
    80002ea4:	06091663          	bnez	s2,80002f10 <bmap+0x92>
    80002ea8:	4108                	lw	a0,0(a0)
    80002eaa:	ea5ff0ef          	jal	80002d4e <balloc>
    80002eae:	0005091b          	sext.w	s2,a0
    80002eb2:	04090f63          	beqz	s2,80002f10 <bmap+0x92>
    80002eb6:	0524a823          	sw	s2,80(s1)
    80002eba:	a899                	j	80002f10 <bmap+0x92>
    80002ebc:	ff45849b          	addw	s1,a1,-12
    80002ec0:	0004871b          	sext.w	a4,s1
    80002ec4:	0ff00793          	li	a5,255
    80002ec8:	06e7eb63          	bltu	a5,a4,80002f3e <bmap+0xc0>
    80002ecc:	08052903          	lw	s2,128(a0)
    80002ed0:	00091b63          	bnez	s2,80002ee6 <bmap+0x68>
    80002ed4:	4108                	lw	a0,0(a0)
    80002ed6:	e79ff0ef          	jal	80002d4e <balloc>
    80002eda:	0005091b          	sext.w	s2,a0
    80002ede:	02090963          	beqz	s2,80002f10 <bmap+0x92>
    80002ee2:	0929a023          	sw	s2,128(s3)
    80002ee6:	85ca                	mv	a1,s2
    80002ee8:	0009a503          	lw	a0,0(s3)
    80002eec:	be7ff0ef          	jal	80002ad2 <bread>
    80002ef0:	8a2a                	mv	s4,a0
    80002ef2:	05850793          	add	a5,a0,88
    80002ef6:	02049593          	sll	a1,s1,0x20
    80002efa:	9181                	srl	a1,a1,0x20
    80002efc:	058a                	sll	a1,a1,0x2
    80002efe:	00b784b3          	add	s1,a5,a1
    80002f02:	0004a903          	lw	s2,0(s1)
    80002f06:	00090e63          	beqz	s2,80002f22 <bmap+0xa4>
    80002f0a:	8552                	mv	a0,s4
    80002f0c:	ce1ff0ef          	jal	80002bec <brelse>
    80002f10:	854a                	mv	a0,s2
    80002f12:	70a2                	ld	ra,40(sp)
    80002f14:	7402                	ld	s0,32(sp)
    80002f16:	64e2                	ld	s1,24(sp)
    80002f18:	6942                	ld	s2,16(sp)
    80002f1a:	69a2                	ld	s3,8(sp)
    80002f1c:	6a02                	ld	s4,0(sp)
    80002f1e:	6145                	add	sp,sp,48
    80002f20:	8082                	ret
    80002f22:	0009a503          	lw	a0,0(s3)
    80002f26:	e29ff0ef          	jal	80002d4e <balloc>
    80002f2a:	0005091b          	sext.w	s2,a0
    80002f2e:	fc090ee3          	beqz	s2,80002f0a <bmap+0x8c>
    80002f32:	0124a023          	sw	s2,0(s1)
    80002f36:	8552                	mv	a0,s4
    80002f38:	5f7000ef          	jal	80003d2e <log_write>
    80002f3c:	b7f9                	j	80002f0a <bmap+0x8c>
    80002f3e:	00004517          	auipc	a0,0x4
    80002f42:	5ca50513          	add	a0,a0,1482 # 80007508 <syscalls+0x140>
    80002f46:	87dfd0ef          	jal	800007c2 <panic>

0000000080002f4a <iget>:
    80002f4a:	7179                	add	sp,sp,-48
    80002f4c:	f406                	sd	ra,40(sp)
    80002f4e:	f022                	sd	s0,32(sp)
    80002f50:	ec26                	sd	s1,24(sp)
    80002f52:	e84a                	sd	s2,16(sp)
    80002f54:	e44e                	sd	s3,8(sp)
    80002f56:	e052                	sd	s4,0(sp)
    80002f58:	1800                	add	s0,sp,48
    80002f5a:	89aa                	mv	s3,a0
    80002f5c:	8a2e                	mv	s4,a1
    80002f5e:	0001b517          	auipc	a0,0x1b
    80002f62:	f2250513          	add	a0,a0,-222 # 8001de80 <itable>
    80002f66:	c51fd0ef          	jal	80000bb6 <acquire>
    80002f6a:	4901                	li	s2,0
    80002f6c:	0001b497          	auipc	s1,0x1b
    80002f70:	f2c48493          	add	s1,s1,-212 # 8001de98 <itable+0x18>
    80002f74:	0001d697          	auipc	a3,0x1d
    80002f78:	9b468693          	add	a3,a3,-1612 # 8001f928 <log>
    80002f7c:	a039                	j	80002f8a <iget+0x40>
    80002f7e:	02090963          	beqz	s2,80002fb0 <iget+0x66>
    80002f82:	08848493          	add	s1,s1,136
    80002f86:	02d48863          	beq	s1,a3,80002fb6 <iget+0x6c>
    80002f8a:	449c                	lw	a5,8(s1)
    80002f8c:	fef059e3          	blez	a5,80002f7e <iget+0x34>
    80002f90:	4098                	lw	a4,0(s1)
    80002f92:	ff3716e3          	bne	a4,s3,80002f7e <iget+0x34>
    80002f96:	40d8                	lw	a4,4(s1)
    80002f98:	ff4713e3          	bne	a4,s4,80002f7e <iget+0x34>
    80002f9c:	2785                	addw	a5,a5,1
    80002f9e:	c49c                	sw	a5,8(s1)
    80002fa0:	0001b517          	auipc	a0,0x1b
    80002fa4:	ee050513          	add	a0,a0,-288 # 8001de80 <itable>
    80002fa8:	ca7fd0ef          	jal	80000c4e <release>
    80002fac:	8926                	mv	s2,s1
    80002fae:	a02d                	j	80002fd8 <iget+0x8e>
    80002fb0:	fbe9                	bnez	a5,80002f82 <iget+0x38>
    80002fb2:	8926                	mv	s2,s1
    80002fb4:	b7f9                	j	80002f82 <iget+0x38>
    80002fb6:	02090a63          	beqz	s2,80002fea <iget+0xa0>
    80002fba:	01392023          	sw	s3,0(s2)
    80002fbe:	01492223          	sw	s4,4(s2)
    80002fc2:	4785                	li	a5,1
    80002fc4:	00f92423          	sw	a5,8(s2)
    80002fc8:	04092023          	sw	zero,64(s2)
    80002fcc:	0001b517          	auipc	a0,0x1b
    80002fd0:	eb450513          	add	a0,a0,-332 # 8001de80 <itable>
    80002fd4:	c7bfd0ef          	jal	80000c4e <release>
    80002fd8:	854a                	mv	a0,s2
    80002fda:	70a2                	ld	ra,40(sp)
    80002fdc:	7402                	ld	s0,32(sp)
    80002fde:	64e2                	ld	s1,24(sp)
    80002fe0:	6942                	ld	s2,16(sp)
    80002fe2:	69a2                	ld	s3,8(sp)
    80002fe4:	6a02                	ld	s4,0(sp)
    80002fe6:	6145                	add	sp,sp,48
    80002fe8:	8082                	ret
    80002fea:	00004517          	auipc	a0,0x4
    80002fee:	53650513          	add	a0,a0,1334 # 80007520 <syscalls+0x158>
    80002ff2:	fd0fd0ef          	jal	800007c2 <panic>

0000000080002ff6 <iinit>:
    80002ff6:	7179                	add	sp,sp,-48
    80002ff8:	f406                	sd	ra,40(sp)
    80002ffa:	f022                	sd	s0,32(sp)
    80002ffc:	ec26                	sd	s1,24(sp)
    80002ffe:	e84a                	sd	s2,16(sp)
    80003000:	e44e                	sd	s3,8(sp)
    80003002:	1800                	add	s0,sp,48
    80003004:	00004597          	auipc	a1,0x4
    80003008:	52c58593          	add	a1,a1,1324 # 80007530 <syscalls+0x168>
    8000300c:	0001b517          	auipc	a0,0x1b
    80003010:	e7450513          	add	a0,a0,-396 # 8001de80 <itable>
    80003014:	b23fd0ef          	jal	80000b36 <initlock>
    80003018:	0001b497          	auipc	s1,0x1b
    8000301c:	e9048493          	add	s1,s1,-368 # 8001dea8 <itable+0x28>
    80003020:	0001d997          	auipc	s3,0x1d
    80003024:	91898993          	add	s3,s3,-1768 # 8001f938 <log+0x10>
    80003028:	00004917          	auipc	s2,0x4
    8000302c:	51090913          	add	s2,s2,1296 # 80007538 <syscalls+0x170>
    80003030:	85ca                	mv	a1,s2
    80003032:	8526                	mv	a0,s1
    80003034:	5d9000ef          	jal	80003e0c <initsleeplock>
    80003038:	08848493          	add	s1,s1,136
    8000303c:	ff349ae3          	bne	s1,s3,80003030 <iinit+0x3a>
    80003040:	70a2                	ld	ra,40(sp)
    80003042:	7402                	ld	s0,32(sp)
    80003044:	64e2                	ld	s1,24(sp)
    80003046:	6942                	ld	s2,16(sp)
    80003048:	69a2                	ld	s3,8(sp)
    8000304a:	6145                	add	sp,sp,48
    8000304c:	8082                	ret

000000008000304e <ialloc>:
    8000304e:	715d                	add	sp,sp,-80
    80003050:	e486                	sd	ra,72(sp)
    80003052:	e0a2                	sd	s0,64(sp)
    80003054:	fc26                	sd	s1,56(sp)
    80003056:	f84a                	sd	s2,48(sp)
    80003058:	f44e                	sd	s3,40(sp)
    8000305a:	f052                	sd	s4,32(sp)
    8000305c:	ec56                	sd	s5,24(sp)
    8000305e:	e85a                	sd	s6,16(sp)
    80003060:	e45e                	sd	s7,8(sp)
    80003062:	0880                	add	s0,sp,80
    80003064:	0001b797          	auipc	a5,0x1b
    80003068:	dfc78793          	add	a5,a5,-516 # 8001de60 <sb>
    8000306c:	47d8                	lw	a4,12(a5)
    8000306e:	4785                	li	a5,1
    80003070:	04e7f663          	bgeu	a5,a4,800030bc <ialloc+0x6e>
    80003074:	8a2a                	mv	s4,a0
    80003076:	8b2e                	mv	s6,a1
    80003078:	4485                	li	s1,1
    8000307a:	0001b997          	auipc	s3,0x1b
    8000307e:	de698993          	add	s3,s3,-538 # 8001de60 <sb>
    80003082:	00048a9b          	sext.w	s5,s1
    80003086:	0044d593          	srl	a1,s1,0x4
    8000308a:	0189a783          	lw	a5,24(s3)
    8000308e:	9dbd                	addw	a1,a1,a5
    80003090:	8552                	mv	a0,s4
    80003092:	a41ff0ef          	jal	80002ad2 <bread>
    80003096:	8baa                	mv	s7,a0
    80003098:	05850913          	add	s2,a0,88
    8000309c:	00f4f793          	and	a5,s1,15
    800030a0:	079a                	sll	a5,a5,0x6
    800030a2:	993e                	add	s2,s2,a5
    800030a4:	00091783          	lh	a5,0(s2)
    800030a8:	cf85                	beqz	a5,800030e0 <ialloc+0x92>
    800030aa:	b43ff0ef          	jal	80002bec <brelse>
    800030ae:	0485                	add	s1,s1,1
    800030b0:	00c9a703          	lw	a4,12(s3)
    800030b4:	0004879b          	sext.w	a5,s1
    800030b8:	fce7e5e3          	bltu	a5,a4,80003082 <ialloc+0x34>
    800030bc:	00004517          	auipc	a0,0x4
    800030c0:	48450513          	add	a0,a0,1156 # 80007540 <syscalls+0x178>
    800030c4:	c30fd0ef          	jal	800004f4 <printf>
    800030c8:	4501                	li	a0,0
    800030ca:	60a6                	ld	ra,72(sp)
    800030cc:	6406                	ld	s0,64(sp)
    800030ce:	74e2                	ld	s1,56(sp)
    800030d0:	7942                	ld	s2,48(sp)
    800030d2:	79a2                	ld	s3,40(sp)
    800030d4:	7a02                	ld	s4,32(sp)
    800030d6:	6ae2                	ld	s5,24(sp)
    800030d8:	6b42                	ld	s6,16(sp)
    800030da:	6ba2                	ld	s7,8(sp)
    800030dc:	6161                	add	sp,sp,80
    800030de:	8082                	ret
    800030e0:	04000613          	li	a2,64
    800030e4:	4581                	li	a1,0
    800030e6:	854a                	mv	a0,s2
    800030e8:	ba3fd0ef          	jal	80000c8a <memset>
    800030ec:	01691023          	sh	s6,0(s2)
    800030f0:	855e                	mv	a0,s7
    800030f2:	43d000ef          	jal	80003d2e <log_write>
    800030f6:	855e                	mv	a0,s7
    800030f8:	af5ff0ef          	jal	80002bec <brelse>
    800030fc:	85d6                	mv	a1,s5
    800030fe:	8552                	mv	a0,s4
    80003100:	e4bff0ef          	jal	80002f4a <iget>
    80003104:	b7d9                	j	800030ca <ialloc+0x7c>

0000000080003106 <iupdate>:
    80003106:	1101                	add	sp,sp,-32
    80003108:	ec06                	sd	ra,24(sp)
    8000310a:	e822                	sd	s0,16(sp)
    8000310c:	e426                	sd	s1,8(sp)
    8000310e:	e04a                	sd	s2,0(sp)
    80003110:	1000                	add	s0,sp,32
    80003112:	84aa                	mv	s1,a0
    80003114:	415c                	lw	a5,4(a0)
    80003116:	0047d79b          	srlw	a5,a5,0x4
    8000311a:	0001b717          	auipc	a4,0x1b
    8000311e:	d4670713          	add	a4,a4,-698 # 8001de60 <sb>
    80003122:	4f0c                	lw	a1,24(a4)
    80003124:	9dbd                	addw	a1,a1,a5
    80003126:	4108                	lw	a0,0(a0)
    80003128:	9abff0ef          	jal	80002ad2 <bread>
    8000312c:	892a                	mv	s2,a0
    8000312e:	05850513          	add	a0,a0,88
    80003132:	40dc                	lw	a5,4(s1)
    80003134:	8bbd                	and	a5,a5,15
    80003136:	079a                	sll	a5,a5,0x6
    80003138:	953e                	add	a0,a0,a5
    8000313a:	04449783          	lh	a5,68(s1)
    8000313e:	00f51023          	sh	a5,0(a0)
    80003142:	04649783          	lh	a5,70(s1)
    80003146:	00f51123          	sh	a5,2(a0)
    8000314a:	04849783          	lh	a5,72(s1)
    8000314e:	00f51223          	sh	a5,4(a0)
    80003152:	04a49783          	lh	a5,74(s1)
    80003156:	00f51323          	sh	a5,6(a0)
    8000315a:	44fc                	lw	a5,76(s1)
    8000315c:	c51c                	sw	a5,8(a0)
    8000315e:	03400613          	li	a2,52
    80003162:	05048593          	add	a1,s1,80
    80003166:	0531                	add	a0,a0,12
    80003168:	b8ffd0ef          	jal	80000cf6 <memmove>
    8000316c:	854a                	mv	a0,s2
    8000316e:	3c1000ef          	jal	80003d2e <log_write>
    80003172:	854a                	mv	a0,s2
    80003174:	a79ff0ef          	jal	80002bec <brelse>
    80003178:	60e2                	ld	ra,24(sp)
    8000317a:	6442                	ld	s0,16(sp)
    8000317c:	64a2                	ld	s1,8(sp)
    8000317e:	6902                	ld	s2,0(sp)
    80003180:	6105                	add	sp,sp,32
    80003182:	8082                	ret

0000000080003184 <idup>:
    80003184:	1101                	add	sp,sp,-32
    80003186:	ec06                	sd	ra,24(sp)
    80003188:	e822                	sd	s0,16(sp)
    8000318a:	e426                	sd	s1,8(sp)
    8000318c:	1000                	add	s0,sp,32
    8000318e:	84aa                	mv	s1,a0
    80003190:	0001b517          	auipc	a0,0x1b
    80003194:	cf050513          	add	a0,a0,-784 # 8001de80 <itable>
    80003198:	a1ffd0ef          	jal	80000bb6 <acquire>
    8000319c:	449c                	lw	a5,8(s1)
    8000319e:	2785                	addw	a5,a5,1
    800031a0:	c49c                	sw	a5,8(s1)
    800031a2:	0001b517          	auipc	a0,0x1b
    800031a6:	cde50513          	add	a0,a0,-802 # 8001de80 <itable>
    800031aa:	aa5fd0ef          	jal	80000c4e <release>
    800031ae:	8526                	mv	a0,s1
    800031b0:	60e2                	ld	ra,24(sp)
    800031b2:	6442                	ld	s0,16(sp)
    800031b4:	64a2                	ld	s1,8(sp)
    800031b6:	6105                	add	sp,sp,32
    800031b8:	8082                	ret

00000000800031ba <ilock>:
    800031ba:	1101                	add	sp,sp,-32
    800031bc:	ec06                	sd	ra,24(sp)
    800031be:	e822                	sd	s0,16(sp)
    800031c0:	e426                	sd	s1,8(sp)
    800031c2:	e04a                	sd	s2,0(sp)
    800031c4:	1000                	add	s0,sp,32
    800031c6:	c105                	beqz	a0,800031e6 <ilock+0x2c>
    800031c8:	84aa                	mv	s1,a0
    800031ca:	451c                	lw	a5,8(a0)
    800031cc:	00f05d63          	blez	a5,800031e6 <ilock+0x2c>
    800031d0:	0541                	add	a0,a0,16
    800031d2:	471000ef          	jal	80003e42 <acquiresleep>
    800031d6:	40bc                	lw	a5,64(s1)
    800031d8:	cf89                	beqz	a5,800031f2 <ilock+0x38>
    800031da:	60e2                	ld	ra,24(sp)
    800031dc:	6442                	ld	s0,16(sp)
    800031de:	64a2                	ld	s1,8(sp)
    800031e0:	6902                	ld	s2,0(sp)
    800031e2:	6105                	add	sp,sp,32
    800031e4:	8082                	ret
    800031e6:	00004517          	auipc	a0,0x4
    800031ea:	37250513          	add	a0,a0,882 # 80007558 <syscalls+0x190>
    800031ee:	dd4fd0ef          	jal	800007c2 <panic>
    800031f2:	40dc                	lw	a5,4(s1)
    800031f4:	0047d79b          	srlw	a5,a5,0x4
    800031f8:	0001b717          	auipc	a4,0x1b
    800031fc:	c6870713          	add	a4,a4,-920 # 8001de60 <sb>
    80003200:	4f0c                	lw	a1,24(a4)
    80003202:	9dbd                	addw	a1,a1,a5
    80003204:	4088                	lw	a0,0(s1)
    80003206:	8cdff0ef          	jal	80002ad2 <bread>
    8000320a:	892a                	mv	s2,a0
    8000320c:	05850593          	add	a1,a0,88
    80003210:	40dc                	lw	a5,4(s1)
    80003212:	8bbd                	and	a5,a5,15
    80003214:	079a                	sll	a5,a5,0x6
    80003216:	95be                	add	a1,a1,a5
    80003218:	00059783          	lh	a5,0(a1)
    8000321c:	04f49223          	sh	a5,68(s1)
    80003220:	00259783          	lh	a5,2(a1)
    80003224:	04f49323          	sh	a5,70(s1)
    80003228:	00459783          	lh	a5,4(a1)
    8000322c:	04f49423          	sh	a5,72(s1)
    80003230:	00659783          	lh	a5,6(a1)
    80003234:	04f49523          	sh	a5,74(s1)
    80003238:	459c                	lw	a5,8(a1)
    8000323a:	c4fc                	sw	a5,76(s1)
    8000323c:	03400613          	li	a2,52
    80003240:	05b1                	add	a1,a1,12
    80003242:	05048513          	add	a0,s1,80
    80003246:	ab1fd0ef          	jal	80000cf6 <memmove>
    8000324a:	854a                	mv	a0,s2
    8000324c:	9a1ff0ef          	jal	80002bec <brelse>
    80003250:	4785                	li	a5,1
    80003252:	c0bc                	sw	a5,64(s1)
    80003254:	04449783          	lh	a5,68(s1)
    80003258:	f3c9                	bnez	a5,800031da <ilock+0x20>
    8000325a:	00004517          	auipc	a0,0x4
    8000325e:	30650513          	add	a0,a0,774 # 80007560 <syscalls+0x198>
    80003262:	d60fd0ef          	jal	800007c2 <panic>

0000000080003266 <iunlock>:
    80003266:	1101                	add	sp,sp,-32
    80003268:	ec06                	sd	ra,24(sp)
    8000326a:	e822                	sd	s0,16(sp)
    8000326c:	e426                	sd	s1,8(sp)
    8000326e:	e04a                	sd	s2,0(sp)
    80003270:	1000                	add	s0,sp,32
    80003272:	c505                	beqz	a0,8000329a <iunlock+0x34>
    80003274:	84aa                	mv	s1,a0
    80003276:	01050913          	add	s2,a0,16
    8000327a:	854a                	mv	a0,s2
    8000327c:	445000ef          	jal	80003ec0 <holdingsleep>
    80003280:	cd09                	beqz	a0,8000329a <iunlock+0x34>
    80003282:	449c                	lw	a5,8(s1)
    80003284:	00f05b63          	blez	a5,8000329a <iunlock+0x34>
    80003288:	854a                	mv	a0,s2
    8000328a:	3ff000ef          	jal	80003e88 <releasesleep>
    8000328e:	60e2                	ld	ra,24(sp)
    80003290:	6442                	ld	s0,16(sp)
    80003292:	64a2                	ld	s1,8(sp)
    80003294:	6902                	ld	s2,0(sp)
    80003296:	6105                	add	sp,sp,32
    80003298:	8082                	ret
    8000329a:	00004517          	auipc	a0,0x4
    8000329e:	2d650513          	add	a0,a0,726 # 80007570 <syscalls+0x1a8>
    800032a2:	d20fd0ef          	jal	800007c2 <panic>

00000000800032a6 <itrunc>:
    800032a6:	7179                	add	sp,sp,-48
    800032a8:	f406                	sd	ra,40(sp)
    800032aa:	f022                	sd	s0,32(sp)
    800032ac:	ec26                	sd	s1,24(sp)
    800032ae:	e84a                	sd	s2,16(sp)
    800032b0:	e44e                	sd	s3,8(sp)
    800032b2:	e052                	sd	s4,0(sp)
    800032b4:	1800                	add	s0,sp,48
    800032b6:	89aa                	mv	s3,a0
    800032b8:	05050493          	add	s1,a0,80
    800032bc:	08050913          	add	s2,a0,128
    800032c0:	a811                	j	800032d4 <itrunc+0x2e>
    800032c2:	0009a503          	lw	a0,0(s3)
    800032c6:	a19ff0ef          	jal	80002cde <bfree>
    800032ca:	0004a023          	sw	zero,0(s1)
    800032ce:	0491                	add	s1,s1,4
    800032d0:	01248563          	beq	s1,s2,800032da <itrunc+0x34>
    800032d4:	408c                	lw	a1,0(s1)
    800032d6:	dde5                	beqz	a1,800032ce <itrunc+0x28>
    800032d8:	b7ed                	j	800032c2 <itrunc+0x1c>
    800032da:	0809a583          	lw	a1,128(s3)
    800032de:	ed91                	bnez	a1,800032fa <itrunc+0x54>
    800032e0:	0409a623          	sw	zero,76(s3)
    800032e4:	854e                	mv	a0,s3
    800032e6:	e21ff0ef          	jal	80003106 <iupdate>
    800032ea:	70a2                	ld	ra,40(sp)
    800032ec:	7402                	ld	s0,32(sp)
    800032ee:	64e2                	ld	s1,24(sp)
    800032f0:	6942                	ld	s2,16(sp)
    800032f2:	69a2                	ld	s3,8(sp)
    800032f4:	6a02                	ld	s4,0(sp)
    800032f6:	6145                	add	sp,sp,48
    800032f8:	8082                	ret
    800032fa:	0009a503          	lw	a0,0(s3)
    800032fe:	fd4ff0ef          	jal	80002ad2 <bread>
    80003302:	8a2a                	mv	s4,a0
    80003304:	05850493          	add	s1,a0,88
    80003308:	45850913          	add	s2,a0,1112
    8000330c:	a801                	j	8000331c <itrunc+0x76>
    8000330e:	0009a503          	lw	a0,0(s3)
    80003312:	9cdff0ef          	jal	80002cde <bfree>
    80003316:	0491                	add	s1,s1,4
    80003318:	01248563          	beq	s1,s2,80003322 <itrunc+0x7c>
    8000331c:	408c                	lw	a1,0(s1)
    8000331e:	dde5                	beqz	a1,80003316 <itrunc+0x70>
    80003320:	b7fd                	j	8000330e <itrunc+0x68>
    80003322:	8552                	mv	a0,s4
    80003324:	8c9ff0ef          	jal	80002bec <brelse>
    80003328:	0809a583          	lw	a1,128(s3)
    8000332c:	0009a503          	lw	a0,0(s3)
    80003330:	9afff0ef          	jal	80002cde <bfree>
    80003334:	0809a023          	sw	zero,128(s3)
    80003338:	b765                	j	800032e0 <itrunc+0x3a>

000000008000333a <iput>:
    8000333a:	1101                	add	sp,sp,-32
    8000333c:	ec06                	sd	ra,24(sp)
    8000333e:	e822                	sd	s0,16(sp)
    80003340:	e426                	sd	s1,8(sp)
    80003342:	e04a                	sd	s2,0(sp)
    80003344:	1000                	add	s0,sp,32
    80003346:	84aa                	mv	s1,a0
    80003348:	0001b517          	auipc	a0,0x1b
    8000334c:	b3850513          	add	a0,a0,-1224 # 8001de80 <itable>
    80003350:	867fd0ef          	jal	80000bb6 <acquire>
    80003354:	4498                	lw	a4,8(s1)
    80003356:	4785                	li	a5,1
    80003358:	02f70163          	beq	a4,a5,8000337a <iput+0x40>
    8000335c:	449c                	lw	a5,8(s1)
    8000335e:	37fd                	addw	a5,a5,-1
    80003360:	c49c                	sw	a5,8(s1)
    80003362:	0001b517          	auipc	a0,0x1b
    80003366:	b1e50513          	add	a0,a0,-1250 # 8001de80 <itable>
    8000336a:	8e5fd0ef          	jal	80000c4e <release>
    8000336e:	60e2                	ld	ra,24(sp)
    80003370:	6442                	ld	s0,16(sp)
    80003372:	64a2                	ld	s1,8(sp)
    80003374:	6902                	ld	s2,0(sp)
    80003376:	6105                	add	sp,sp,32
    80003378:	8082                	ret
    8000337a:	40bc                	lw	a5,64(s1)
    8000337c:	d3e5                	beqz	a5,8000335c <iput+0x22>
    8000337e:	04a49783          	lh	a5,74(s1)
    80003382:	ffe9                	bnez	a5,8000335c <iput+0x22>
    80003384:	01048913          	add	s2,s1,16
    80003388:	854a                	mv	a0,s2
    8000338a:	2b9000ef          	jal	80003e42 <acquiresleep>
    8000338e:	0001b517          	auipc	a0,0x1b
    80003392:	af250513          	add	a0,a0,-1294 # 8001de80 <itable>
    80003396:	8b9fd0ef          	jal	80000c4e <release>
    8000339a:	8526                	mv	a0,s1
    8000339c:	f0bff0ef          	jal	800032a6 <itrunc>
    800033a0:	04049223          	sh	zero,68(s1)
    800033a4:	8526                	mv	a0,s1
    800033a6:	d61ff0ef          	jal	80003106 <iupdate>
    800033aa:	0404a023          	sw	zero,64(s1)
    800033ae:	854a                	mv	a0,s2
    800033b0:	2d9000ef          	jal	80003e88 <releasesleep>
    800033b4:	0001b517          	auipc	a0,0x1b
    800033b8:	acc50513          	add	a0,a0,-1332 # 8001de80 <itable>
    800033bc:	ffafd0ef          	jal	80000bb6 <acquire>
    800033c0:	bf71                	j	8000335c <iput+0x22>

00000000800033c2 <iunlockput>:
    800033c2:	1101                	add	sp,sp,-32
    800033c4:	ec06                	sd	ra,24(sp)
    800033c6:	e822                	sd	s0,16(sp)
    800033c8:	e426                	sd	s1,8(sp)
    800033ca:	1000                	add	s0,sp,32
    800033cc:	84aa                	mv	s1,a0
    800033ce:	e99ff0ef          	jal	80003266 <iunlock>
    800033d2:	8526                	mv	a0,s1
    800033d4:	f67ff0ef          	jal	8000333a <iput>
    800033d8:	60e2                	ld	ra,24(sp)
    800033da:	6442                	ld	s0,16(sp)
    800033dc:	64a2                	ld	s1,8(sp)
    800033de:	6105                	add	sp,sp,32
    800033e0:	8082                	ret

00000000800033e2 <ireclaim>:
    800033e2:	0001b797          	auipc	a5,0x1b
    800033e6:	a7e78793          	add	a5,a5,-1410 # 8001de60 <sb>
    800033ea:	47d8                	lw	a4,12(a5)
    800033ec:	4785                	li	a5,1
    800033ee:	0ae7ff63          	bgeu	a5,a4,800034ac <ireclaim+0xca>
    800033f2:	7139                	add	sp,sp,-64
    800033f4:	fc06                	sd	ra,56(sp)
    800033f6:	f822                	sd	s0,48(sp)
    800033f8:	f426                	sd	s1,40(sp)
    800033fa:	f04a                	sd	s2,32(sp)
    800033fc:	ec4e                	sd	s3,24(sp)
    800033fe:	e852                	sd	s4,16(sp)
    80003400:	e456                	sd	s5,8(sp)
    80003402:	e05a                	sd	s6,0(sp)
    80003404:	0080                	add	s0,sp,64
    80003406:	4485                	li	s1,1
    80003408:	00050a1b          	sext.w	s4,a0
    8000340c:	0001b997          	auipc	s3,0x1b
    80003410:	a5498993          	add	s3,s3,-1452 # 8001de60 <sb>
    80003414:	00004a97          	auipc	s5,0x4
    80003418:	164a8a93          	add	s5,s5,356 # 80007578 <syscalls+0x1b0>
    8000341c:	a099                	j	80003462 <ireclaim+0x80>
    8000341e:	85ca                	mv	a1,s2
    80003420:	8556                	mv	a0,s5
    80003422:	8d2fd0ef          	jal	800004f4 <printf>
    80003426:	85ca                	mv	a1,s2
    80003428:	8552                	mv	a0,s4
    8000342a:	b21ff0ef          	jal	80002f4a <iget>
    8000342e:	892a                	mv	s2,a0
    80003430:	855a                	mv	a0,s6
    80003432:	fbaff0ef          	jal	80002bec <brelse>
    80003436:	00090f63          	beqz	s2,80003454 <ireclaim+0x72>
    8000343a:	76a000ef          	jal	80003ba4 <begin_op>
    8000343e:	854a                	mv	a0,s2
    80003440:	d7bff0ef          	jal	800031ba <ilock>
    80003444:	854a                	mv	a0,s2
    80003446:	e21ff0ef          	jal	80003266 <iunlock>
    8000344a:	854a                	mv	a0,s2
    8000344c:	eefff0ef          	jal	8000333a <iput>
    80003450:	7c4000ef          	jal	80003c14 <end_op>
    80003454:	0485                	add	s1,s1,1
    80003456:	00c9a703          	lw	a4,12(s3)
    8000345a:	0004879b          	sext.w	a5,s1
    8000345e:	02e7fd63          	bgeu	a5,a4,80003498 <ireclaim+0xb6>
    80003462:	0004891b          	sext.w	s2,s1
    80003466:	0044d593          	srl	a1,s1,0x4
    8000346a:	0189a783          	lw	a5,24(s3)
    8000346e:	9dbd                	addw	a1,a1,a5
    80003470:	8552                	mv	a0,s4
    80003472:	e60ff0ef          	jal	80002ad2 <bread>
    80003476:	8b2a                	mv	s6,a0
    80003478:	05850793          	add	a5,a0,88
    8000347c:	00f97713          	and	a4,s2,15
    80003480:	071a                	sll	a4,a4,0x6
    80003482:	97ba                	add	a5,a5,a4
    80003484:	00079703          	lh	a4,0(a5)
    80003488:	c701                	beqz	a4,80003490 <ireclaim+0xae>
    8000348a:	00679783          	lh	a5,6(a5)
    8000348e:	dbc1                	beqz	a5,8000341e <ireclaim+0x3c>
    80003490:	855a                	mv	a0,s6
    80003492:	f5aff0ef          	jal	80002bec <brelse>
    80003496:	bf7d                	j	80003454 <ireclaim+0x72>
    80003498:	70e2                	ld	ra,56(sp)
    8000349a:	7442                	ld	s0,48(sp)
    8000349c:	74a2                	ld	s1,40(sp)
    8000349e:	7902                	ld	s2,32(sp)
    800034a0:	69e2                	ld	s3,24(sp)
    800034a2:	6a42                	ld	s4,16(sp)
    800034a4:	6aa2                	ld	s5,8(sp)
    800034a6:	6b02                	ld	s6,0(sp)
    800034a8:	6121                	add	sp,sp,64
    800034aa:	8082                	ret
    800034ac:	8082                	ret

00000000800034ae <fsinit>:
    800034ae:	7179                	add	sp,sp,-48
    800034b0:	f406                	sd	ra,40(sp)
    800034b2:	f022                	sd	s0,32(sp)
    800034b4:	ec26                	sd	s1,24(sp)
    800034b6:	e84a                	sd	s2,16(sp)
    800034b8:	e44e                	sd	s3,8(sp)
    800034ba:	1800                	add	s0,sp,48
    800034bc:	84aa                	mv	s1,a0
    800034be:	4585                	li	a1,1
    800034c0:	e12ff0ef          	jal	80002ad2 <bread>
    800034c4:	89aa                	mv	s3,a0
    800034c6:	0001b917          	auipc	s2,0x1b
    800034ca:	99a90913          	add	s2,s2,-1638 # 8001de60 <sb>
    800034ce:	02000613          	li	a2,32
    800034d2:	05850593          	add	a1,a0,88
    800034d6:	854a                	mv	a0,s2
    800034d8:	81ffd0ef          	jal	80000cf6 <memmove>
    800034dc:	854e                	mv	a0,s3
    800034de:	f0eff0ef          	jal	80002bec <brelse>
    800034e2:	00092703          	lw	a4,0(s2)
    800034e6:	102037b7          	lui	a5,0x10203
    800034ea:	04078793          	add	a5,a5,64 # 10203040 <_entry-0x6fdfcfc0>
    800034ee:	02f71363          	bne	a4,a5,80003514 <fsinit+0x66>
    800034f2:	0001b597          	auipc	a1,0x1b
    800034f6:	96e58593          	add	a1,a1,-1682 # 8001de60 <sb>
    800034fa:	8526                	mv	a0,s1
    800034fc:	61e000ef          	jal	80003b1a <initlog>
    80003500:	8526                	mv	a0,s1
    80003502:	ee1ff0ef          	jal	800033e2 <ireclaim>
    80003506:	70a2                	ld	ra,40(sp)
    80003508:	7402                	ld	s0,32(sp)
    8000350a:	64e2                	ld	s1,24(sp)
    8000350c:	6942                	ld	s2,16(sp)
    8000350e:	69a2                	ld	s3,8(sp)
    80003510:	6145                	add	sp,sp,48
    80003512:	8082                	ret
    80003514:	00004517          	auipc	a0,0x4
    80003518:	08450513          	add	a0,a0,132 # 80007598 <syscalls+0x1d0>
    8000351c:	aa6fd0ef          	jal	800007c2 <panic>

0000000080003520 <stati>:
    80003520:	1141                	add	sp,sp,-16
    80003522:	e422                	sd	s0,8(sp)
    80003524:	0800                	add	s0,sp,16
    80003526:	411c                	lw	a5,0(a0)
    80003528:	c19c                	sw	a5,0(a1)
    8000352a:	415c                	lw	a5,4(a0)
    8000352c:	c1dc                	sw	a5,4(a1)
    8000352e:	04451783          	lh	a5,68(a0)
    80003532:	00f59423          	sh	a5,8(a1)
    80003536:	04a51783          	lh	a5,74(a0)
    8000353a:	00f59523          	sh	a5,10(a1)
    8000353e:	04c56783          	lwu	a5,76(a0)
    80003542:	e99c                	sd	a5,16(a1)
    80003544:	6422                	ld	s0,8(sp)
    80003546:	0141                	add	sp,sp,16
    80003548:	8082                	ret

000000008000354a <readi>:
    8000354a:	457c                	lw	a5,76(a0)
    8000354c:	0cd7ef63          	bltu	a5,a3,8000362a <readi+0xe0>
    80003550:	7159                	add	sp,sp,-112
    80003552:	f486                	sd	ra,104(sp)
    80003554:	f0a2                	sd	s0,96(sp)
    80003556:	eca6                	sd	s1,88(sp)
    80003558:	e8ca                	sd	s2,80(sp)
    8000355a:	e4ce                	sd	s3,72(sp)
    8000355c:	e0d2                	sd	s4,64(sp)
    8000355e:	fc56                	sd	s5,56(sp)
    80003560:	f85a                	sd	s6,48(sp)
    80003562:	f45e                	sd	s7,40(sp)
    80003564:	f062                	sd	s8,32(sp)
    80003566:	ec66                	sd	s9,24(sp)
    80003568:	e86a                	sd	s10,16(sp)
    8000356a:	e46e                	sd	s11,8(sp)
    8000356c:	1880                	add	s0,sp,112
    8000356e:	8baa                	mv	s7,a0
    80003570:	8c2e                	mv	s8,a1
    80003572:	8a32                	mv	s4,a2
    80003574:	84b6                	mv	s1,a3
    80003576:	8b3a                	mv	s6,a4
    80003578:	9f35                	addw	a4,a4,a3
    8000357a:	4501                	li	a0,0
    8000357c:	08d76663          	bltu	a4,a3,80003608 <readi+0xbe>
    80003580:	00e7f463          	bgeu	a5,a4,80003588 <readi+0x3e>
    80003584:	40d78b3b          	subw	s6,a5,a3
    80003588:	080b0f63          	beqz	s6,80003626 <readi+0xdc>
    8000358c:	4901                	li	s2,0
    8000358e:	40000d13          	li	s10,1024
    80003592:	5cfd                	li	s9,-1
    80003594:	a80d                	j	800035c6 <readi+0x7c>
    80003596:	02099d93          	sll	s11,s3,0x20
    8000359a:	020ddd93          	srl	s11,s11,0x20
    8000359e:	058a8613          	add	a2,s5,88
    800035a2:	86ee                	mv	a3,s11
    800035a4:	963a                	add	a2,a2,a4
    800035a6:	85d2                	mv	a1,s4
    800035a8:	8562                	mv	a0,s8
    800035aa:	c49fe0ef          	jal	800021f2 <either_copyout>
    800035ae:	05950763          	beq	a0,s9,800035fc <readi+0xb2>
    800035b2:	8556                	mv	a0,s5
    800035b4:	e38ff0ef          	jal	80002bec <brelse>
    800035b8:	0129893b          	addw	s2,s3,s2
    800035bc:	009984bb          	addw	s1,s3,s1
    800035c0:	9a6e                	add	s4,s4,s11
    800035c2:	05697163          	bgeu	s2,s6,80003604 <readi+0xba>
    800035c6:	00a4d59b          	srlw	a1,s1,0xa
    800035ca:	855e                	mv	a0,s7
    800035cc:	8b3ff0ef          	jal	80002e7e <bmap>
    800035d0:	0005059b          	sext.w	a1,a0
    800035d4:	c985                	beqz	a1,80003604 <readi+0xba>
    800035d6:	000ba503          	lw	a0,0(s7)
    800035da:	cf8ff0ef          	jal	80002ad2 <bread>
    800035de:	8aaa                	mv	s5,a0
    800035e0:	3ff4f713          	and	a4,s1,1023
    800035e4:	40ed07bb          	subw	a5,s10,a4
    800035e8:	412b06bb          	subw	a3,s6,s2
    800035ec:	89be                	mv	s3,a5
    800035ee:	2781                	sext.w	a5,a5
    800035f0:	0006861b          	sext.w	a2,a3
    800035f4:	faf671e3          	bgeu	a2,a5,80003596 <readi+0x4c>
    800035f8:	89b6                	mv	s3,a3
    800035fa:	bf71                	j	80003596 <readi+0x4c>
    800035fc:	8556                	mv	a0,s5
    800035fe:	deeff0ef          	jal	80002bec <brelse>
    80003602:	597d                	li	s2,-1
    80003604:	0009051b          	sext.w	a0,s2
    80003608:	70a6                	ld	ra,104(sp)
    8000360a:	7406                	ld	s0,96(sp)
    8000360c:	64e6                	ld	s1,88(sp)
    8000360e:	6946                	ld	s2,80(sp)
    80003610:	69a6                	ld	s3,72(sp)
    80003612:	6a06                	ld	s4,64(sp)
    80003614:	7ae2                	ld	s5,56(sp)
    80003616:	7b42                	ld	s6,48(sp)
    80003618:	7ba2                	ld	s7,40(sp)
    8000361a:	7c02                	ld	s8,32(sp)
    8000361c:	6ce2                	ld	s9,24(sp)
    8000361e:	6d42                	ld	s10,16(sp)
    80003620:	6da2                	ld	s11,8(sp)
    80003622:	6165                	add	sp,sp,112
    80003624:	8082                	ret
    80003626:	895a                	mv	s2,s6
    80003628:	bff1                	j	80003604 <readi+0xba>
    8000362a:	4501                	li	a0,0
    8000362c:	8082                	ret

000000008000362e <writei>:
    8000362e:	457c                	lw	a5,76(a0)
    80003630:	0ed7ea63          	bltu	a5,a3,80003724 <writei+0xf6>
    80003634:	7159                	add	sp,sp,-112
    80003636:	f486                	sd	ra,104(sp)
    80003638:	f0a2                	sd	s0,96(sp)
    8000363a:	eca6                	sd	s1,88(sp)
    8000363c:	e8ca                	sd	s2,80(sp)
    8000363e:	e4ce                	sd	s3,72(sp)
    80003640:	e0d2                	sd	s4,64(sp)
    80003642:	fc56                	sd	s5,56(sp)
    80003644:	f85a                	sd	s6,48(sp)
    80003646:	f45e                	sd	s7,40(sp)
    80003648:	f062                	sd	s8,32(sp)
    8000364a:	ec66                	sd	s9,24(sp)
    8000364c:	e86a                	sd	s10,16(sp)
    8000364e:	e46e                	sd	s11,8(sp)
    80003650:	1880                	add	s0,sp,112
    80003652:	8b2a                	mv	s6,a0
    80003654:	8c2e                	mv	s8,a1
    80003656:	8ab2                	mv	s5,a2
    80003658:	84b6                	mv	s1,a3
    8000365a:	8bba                	mv	s7,a4
    8000365c:	00e687bb          	addw	a5,a3,a4
    80003660:	0cd7e463          	bltu	a5,a3,80003728 <writei+0xfa>
    80003664:	00043737          	lui	a4,0x43
    80003668:	0cf76263          	bltu	a4,a5,8000372c <writei+0xfe>
    8000366c:	0a0b8a63          	beqz	s7,80003720 <writei+0xf2>
    80003670:	4981                	li	s3,0
    80003672:	40000d13          	li	s10,1024
    80003676:	5cfd                	li	s9,-1
    80003678:	a825                	j	800036b0 <writei+0x82>
    8000367a:	02091d93          	sll	s11,s2,0x20
    8000367e:	020ddd93          	srl	s11,s11,0x20
    80003682:	058a0513          	add	a0,s4,88 # 2058 <_entry-0x7fffdfa8>
    80003686:	86ee                	mv	a3,s11
    80003688:	8656                	mv	a2,s5
    8000368a:	85e2                	mv	a1,s8
    8000368c:	953a                	add	a0,a0,a4
    8000368e:	baffe0ef          	jal	8000223c <either_copyin>
    80003692:	05950a63          	beq	a0,s9,800036e6 <writei+0xb8>
    80003696:	8552                	mv	a0,s4
    80003698:	696000ef          	jal	80003d2e <log_write>
    8000369c:	8552                	mv	a0,s4
    8000369e:	d4eff0ef          	jal	80002bec <brelse>
    800036a2:	013909bb          	addw	s3,s2,s3
    800036a6:	009904bb          	addw	s1,s2,s1
    800036aa:	9aee                	add	s5,s5,s11
    800036ac:	0579f063          	bgeu	s3,s7,800036ec <writei+0xbe>
    800036b0:	00a4d59b          	srlw	a1,s1,0xa
    800036b4:	855a                	mv	a0,s6
    800036b6:	fc8ff0ef          	jal	80002e7e <bmap>
    800036ba:	0005059b          	sext.w	a1,a0
    800036be:	c59d                	beqz	a1,800036ec <writei+0xbe>
    800036c0:	000b2503          	lw	a0,0(s6)
    800036c4:	c0eff0ef          	jal	80002ad2 <bread>
    800036c8:	8a2a                	mv	s4,a0
    800036ca:	3ff4f713          	and	a4,s1,1023
    800036ce:	40ed07bb          	subw	a5,s10,a4
    800036d2:	413b86bb          	subw	a3,s7,s3
    800036d6:	893e                	mv	s2,a5
    800036d8:	2781                	sext.w	a5,a5
    800036da:	0006861b          	sext.w	a2,a3
    800036de:	f8f67ee3          	bgeu	a2,a5,8000367a <writei+0x4c>
    800036e2:	8936                	mv	s2,a3
    800036e4:	bf59                	j	8000367a <writei+0x4c>
    800036e6:	8552                	mv	a0,s4
    800036e8:	d04ff0ef          	jal	80002bec <brelse>
    800036ec:	04cb2783          	lw	a5,76(s6)
    800036f0:	0097f463          	bgeu	a5,s1,800036f8 <writei+0xca>
    800036f4:	049b2623          	sw	s1,76(s6)
    800036f8:	855a                	mv	a0,s6
    800036fa:	a0dff0ef          	jal	80003106 <iupdate>
    800036fe:	0009851b          	sext.w	a0,s3
    80003702:	70a6                	ld	ra,104(sp)
    80003704:	7406                	ld	s0,96(sp)
    80003706:	64e6                	ld	s1,88(sp)
    80003708:	6946                	ld	s2,80(sp)
    8000370a:	69a6                	ld	s3,72(sp)
    8000370c:	6a06                	ld	s4,64(sp)
    8000370e:	7ae2                	ld	s5,56(sp)
    80003710:	7b42                	ld	s6,48(sp)
    80003712:	7ba2                	ld	s7,40(sp)
    80003714:	7c02                	ld	s8,32(sp)
    80003716:	6ce2                	ld	s9,24(sp)
    80003718:	6d42                	ld	s10,16(sp)
    8000371a:	6da2                	ld	s11,8(sp)
    8000371c:	6165                	add	sp,sp,112
    8000371e:	8082                	ret
    80003720:	89de                	mv	s3,s7
    80003722:	bfd9                	j	800036f8 <writei+0xca>
    80003724:	557d                	li	a0,-1
    80003726:	8082                	ret
    80003728:	557d                	li	a0,-1
    8000372a:	bfe1                	j	80003702 <writei+0xd4>
    8000372c:	557d                	li	a0,-1
    8000372e:	bfd1                	j	80003702 <writei+0xd4>

0000000080003730 <namecmp>:
    80003730:	1141                	add	sp,sp,-16
    80003732:	e406                	sd	ra,8(sp)
    80003734:	e022                	sd	s0,0(sp)
    80003736:	0800                	add	s0,sp,16
    80003738:	4639                	li	a2,14
    8000373a:	e30fd0ef          	jal	80000d6a <strncmp>
    8000373e:	60a2                	ld	ra,8(sp)
    80003740:	6402                	ld	s0,0(sp)
    80003742:	0141                	add	sp,sp,16
    80003744:	8082                	ret

0000000080003746 <dirlookup>:
    80003746:	7139                	add	sp,sp,-64
    80003748:	fc06                	sd	ra,56(sp)
    8000374a:	f822                	sd	s0,48(sp)
    8000374c:	f426                	sd	s1,40(sp)
    8000374e:	f04a                	sd	s2,32(sp)
    80003750:	ec4e                	sd	s3,24(sp)
    80003752:	e852                	sd	s4,16(sp)
    80003754:	0080                	add	s0,sp,64
    80003756:	04451703          	lh	a4,68(a0)
    8000375a:	4785                	li	a5,1
    8000375c:	00f71a63          	bne	a4,a5,80003770 <dirlookup+0x2a>
    80003760:	892a                	mv	s2,a0
    80003762:	89ae                	mv	s3,a1
    80003764:	8a32                	mv	s4,a2
    80003766:	457c                	lw	a5,76(a0)
    80003768:	4481                	li	s1,0
    8000376a:	4501                	li	a0,0
    8000376c:	e39d                	bnez	a5,80003792 <dirlookup+0x4c>
    8000376e:	a095                	j	800037d2 <dirlookup+0x8c>
    80003770:	00004517          	auipc	a0,0x4
    80003774:	e4050513          	add	a0,a0,-448 # 800075b0 <syscalls+0x1e8>
    80003778:	84afd0ef          	jal	800007c2 <panic>
    8000377c:	00004517          	auipc	a0,0x4
    80003780:	e4c50513          	add	a0,a0,-436 # 800075c8 <syscalls+0x200>
    80003784:	83efd0ef          	jal	800007c2 <panic>
    80003788:	24c1                	addw	s1,s1,16
    8000378a:	04c92783          	lw	a5,76(s2)
    8000378e:	04f4f163          	bgeu	s1,a5,800037d0 <dirlookup+0x8a>
    80003792:	4741                	li	a4,16
    80003794:	86a6                	mv	a3,s1
    80003796:	fc040613          	add	a2,s0,-64
    8000379a:	4581                	li	a1,0
    8000379c:	854a                	mv	a0,s2
    8000379e:	dadff0ef          	jal	8000354a <readi>
    800037a2:	47c1                	li	a5,16
    800037a4:	fcf51ce3          	bne	a0,a5,8000377c <dirlookup+0x36>
    800037a8:	fc045783          	lhu	a5,-64(s0)
    800037ac:	dff1                	beqz	a5,80003788 <dirlookup+0x42>
    800037ae:	fc240593          	add	a1,s0,-62
    800037b2:	854e                	mv	a0,s3
    800037b4:	f7dff0ef          	jal	80003730 <namecmp>
    800037b8:	f961                	bnez	a0,80003788 <dirlookup+0x42>
    800037ba:	000a0463          	beqz	s4,800037c2 <dirlookup+0x7c>
    800037be:	009a2023          	sw	s1,0(s4)
    800037c2:	fc045583          	lhu	a1,-64(s0)
    800037c6:	00092503          	lw	a0,0(s2)
    800037ca:	f80ff0ef          	jal	80002f4a <iget>
    800037ce:	a011                	j	800037d2 <dirlookup+0x8c>
    800037d0:	4501                	li	a0,0
    800037d2:	70e2                	ld	ra,56(sp)
    800037d4:	7442                	ld	s0,48(sp)
    800037d6:	74a2                	ld	s1,40(sp)
    800037d8:	7902                	ld	s2,32(sp)
    800037da:	69e2                	ld	s3,24(sp)
    800037dc:	6a42                	ld	s4,16(sp)
    800037de:	6121                	add	sp,sp,64
    800037e0:	8082                	ret

00000000800037e2 <namex>:
    800037e2:	711d                	add	sp,sp,-96
    800037e4:	ec86                	sd	ra,88(sp)
    800037e6:	e8a2                	sd	s0,80(sp)
    800037e8:	e4a6                	sd	s1,72(sp)
    800037ea:	e0ca                	sd	s2,64(sp)
    800037ec:	fc4e                	sd	s3,56(sp)
    800037ee:	f852                	sd	s4,48(sp)
    800037f0:	f456                	sd	s5,40(sp)
    800037f2:	f05a                	sd	s6,32(sp)
    800037f4:	ec5e                	sd	s7,24(sp)
    800037f6:	e862                	sd	s8,16(sp)
    800037f8:	e466                	sd	s9,8(sp)
    800037fa:	1080                	add	s0,sp,96
    800037fc:	84aa                	mv	s1,a0
    800037fe:	8bae                	mv	s7,a1
    80003800:	8ab2                	mv	s5,a2
    80003802:	00054703          	lbu	a4,0(a0)
    80003806:	02f00793          	li	a5,47
    8000380a:	00f70f63          	beq	a4,a5,80003828 <namex+0x46>
    8000380e:	87efe0ef          	jal	8000188c <myproc>
    80003812:	15053503          	ld	a0,336(a0)
    80003816:	96fff0ef          	jal	80003184 <idup>
    8000381a:	89aa                	mv	s3,a0
    8000381c:	02f00913          	li	s2,47
    80003820:	4b01                	li	s6,0
    80003822:	4cb5                	li	s9,13
    80003824:	4c05                	li	s8,1
    80003826:	a861                	j	800038be <namex+0xdc>
    80003828:	4585                	li	a1,1
    8000382a:	4505                	li	a0,1
    8000382c:	f1eff0ef          	jal	80002f4a <iget>
    80003830:	89aa                	mv	s3,a0
    80003832:	b7ed                	j	8000381c <namex+0x3a>
    80003834:	854e                	mv	a0,s3
    80003836:	b8dff0ef          	jal	800033c2 <iunlockput>
    8000383a:	4981                	li	s3,0
    8000383c:	854e                	mv	a0,s3
    8000383e:	60e6                	ld	ra,88(sp)
    80003840:	6446                	ld	s0,80(sp)
    80003842:	64a6                	ld	s1,72(sp)
    80003844:	6906                	ld	s2,64(sp)
    80003846:	79e2                	ld	s3,56(sp)
    80003848:	7a42                	ld	s4,48(sp)
    8000384a:	7aa2                	ld	s5,40(sp)
    8000384c:	7b02                	ld	s6,32(sp)
    8000384e:	6be2                	ld	s7,24(sp)
    80003850:	6c42                	ld	s8,16(sp)
    80003852:	6ca2                	ld	s9,8(sp)
    80003854:	6125                	add	sp,sp,96
    80003856:	8082                	ret
    80003858:	854e                	mv	a0,s3
    8000385a:	a0dff0ef          	jal	80003266 <iunlock>
    8000385e:	bff9                	j	8000383c <namex+0x5a>
    80003860:	854e                	mv	a0,s3
    80003862:	b61ff0ef          	jal	800033c2 <iunlockput>
    80003866:	89d2                	mv	s3,s4
    80003868:	bfd1                	j	8000383c <namex+0x5a>
    8000386a:	40b48633          	sub	a2,s1,a1
    8000386e:	00060a1b          	sext.w	s4,a2
    80003872:	074cde63          	bge	s9,s4,800038ee <namex+0x10c>
    80003876:	4639                	li	a2,14
    80003878:	8556                	mv	a0,s5
    8000387a:	c7cfd0ef          	jal	80000cf6 <memmove>
    8000387e:	0004c783          	lbu	a5,0(s1)
    80003882:	01279763          	bne	a5,s2,80003890 <namex+0xae>
    80003886:	0485                	add	s1,s1,1
    80003888:	0004c783          	lbu	a5,0(s1)
    8000388c:	ff278de3          	beq	a5,s2,80003886 <namex+0xa4>
    80003890:	854e                	mv	a0,s3
    80003892:	929ff0ef          	jal	800031ba <ilock>
    80003896:	04499783          	lh	a5,68(s3)
    8000389a:	f9879de3          	bne	a5,s8,80003834 <namex+0x52>
    8000389e:	000b8563          	beqz	s7,800038a8 <namex+0xc6>
    800038a2:	0004c783          	lbu	a5,0(s1)
    800038a6:	dbcd                	beqz	a5,80003858 <namex+0x76>
    800038a8:	865a                	mv	a2,s6
    800038aa:	85d6                	mv	a1,s5
    800038ac:	854e                	mv	a0,s3
    800038ae:	e99ff0ef          	jal	80003746 <dirlookup>
    800038b2:	8a2a                	mv	s4,a0
    800038b4:	d555                	beqz	a0,80003860 <namex+0x7e>
    800038b6:	854e                	mv	a0,s3
    800038b8:	b0bff0ef          	jal	800033c2 <iunlockput>
    800038bc:	89d2                	mv	s3,s4
    800038be:	0004c783          	lbu	a5,0(s1)
    800038c2:	05279963          	bne	a5,s2,80003914 <namex+0x132>
    800038c6:	0485                	add	s1,s1,1
    800038c8:	0004c783          	lbu	a5,0(s1)
    800038cc:	ff278de3          	beq	a5,s2,800038c6 <namex+0xe4>
    800038d0:	cb9d                	beqz	a5,80003906 <namex+0x124>
    800038d2:	01278b63          	beq	a5,s2,800038e8 <namex+0x106>
    800038d6:	c785                	beqz	a5,800038fe <namex+0x11c>
    800038d8:	85a6                	mv	a1,s1
    800038da:	0485                	add	s1,s1,1
    800038dc:	0004c783          	lbu	a5,0(s1)
    800038e0:	f92785e3          	beq	a5,s2,8000386a <namex+0x88>
    800038e4:	fbfd                	bnez	a5,800038da <namex+0xf8>
    800038e6:	b751                	j	8000386a <namex+0x88>
    800038e8:	85a6                	mv	a1,s1
    800038ea:	8a5a                	mv	s4,s6
    800038ec:	865a                	mv	a2,s6
    800038ee:	2601                	sext.w	a2,a2
    800038f0:	8556                	mv	a0,s5
    800038f2:	c04fd0ef          	jal	80000cf6 <memmove>
    800038f6:	9a56                	add	s4,s4,s5
    800038f8:	000a0023          	sb	zero,0(s4)
    800038fc:	b749                	j	8000387e <namex+0x9c>
    800038fe:	85a6                	mv	a1,s1
    80003900:	8a5a                	mv	s4,s6
    80003902:	865a                	mv	a2,s6
    80003904:	b7ed                	j	800038ee <namex+0x10c>
    80003906:	f20b8be3          	beqz	s7,8000383c <namex+0x5a>
    8000390a:	854e                	mv	a0,s3
    8000390c:	a2fff0ef          	jal	8000333a <iput>
    80003910:	4981                	li	s3,0
    80003912:	b72d                	j	8000383c <namex+0x5a>
    80003914:	dbed                	beqz	a5,80003906 <namex+0x124>
    80003916:	85a6                	mv	a1,s1
    80003918:	b7c9                	j	800038da <namex+0xf8>

000000008000391a <dirlink>:
    8000391a:	7139                	add	sp,sp,-64
    8000391c:	fc06                	sd	ra,56(sp)
    8000391e:	f822                	sd	s0,48(sp)
    80003920:	f426                	sd	s1,40(sp)
    80003922:	f04a                	sd	s2,32(sp)
    80003924:	ec4e                	sd	s3,24(sp)
    80003926:	e852                	sd	s4,16(sp)
    80003928:	0080                	add	s0,sp,64
    8000392a:	892a                	mv	s2,a0
    8000392c:	8a2e                	mv	s4,a1
    8000392e:	89b2                	mv	s3,a2
    80003930:	4601                	li	a2,0
    80003932:	e15ff0ef          	jal	80003746 <dirlookup>
    80003936:	e52d                	bnez	a0,800039a0 <dirlink+0x86>
    80003938:	04c92483          	lw	s1,76(s2)
    8000393c:	c48d                	beqz	s1,80003966 <dirlink+0x4c>
    8000393e:	4481                	li	s1,0
    80003940:	4741                	li	a4,16
    80003942:	86a6                	mv	a3,s1
    80003944:	fc040613          	add	a2,s0,-64
    80003948:	4581                	li	a1,0
    8000394a:	854a                	mv	a0,s2
    8000394c:	bffff0ef          	jal	8000354a <readi>
    80003950:	47c1                	li	a5,16
    80003952:	04f51b63          	bne	a0,a5,800039a8 <dirlink+0x8e>
    80003956:	fc045783          	lhu	a5,-64(s0)
    8000395a:	c791                	beqz	a5,80003966 <dirlink+0x4c>
    8000395c:	24c1                	addw	s1,s1,16
    8000395e:	04c92783          	lw	a5,76(s2)
    80003962:	fcf4efe3          	bltu	s1,a5,80003940 <dirlink+0x26>
    80003966:	4639                	li	a2,14
    80003968:	85d2                	mv	a1,s4
    8000396a:	fc240513          	add	a0,s0,-62
    8000396e:	c4cfd0ef          	jal	80000dba <strncpy>
    80003972:	fd341023          	sh	s3,-64(s0)
    80003976:	4741                	li	a4,16
    80003978:	86a6                	mv	a3,s1
    8000397a:	fc040613          	add	a2,s0,-64
    8000397e:	4581                	li	a1,0
    80003980:	854a                	mv	a0,s2
    80003982:	cadff0ef          	jal	8000362e <writei>
    80003986:	1541                	add	a0,a0,-16
    80003988:	00a03533          	snez	a0,a0
    8000398c:	40a00533          	neg	a0,a0
    80003990:	70e2                	ld	ra,56(sp)
    80003992:	7442                	ld	s0,48(sp)
    80003994:	74a2                	ld	s1,40(sp)
    80003996:	7902                	ld	s2,32(sp)
    80003998:	69e2                	ld	s3,24(sp)
    8000399a:	6a42                	ld	s4,16(sp)
    8000399c:	6121                	add	sp,sp,64
    8000399e:	8082                	ret
    800039a0:	99bff0ef          	jal	8000333a <iput>
    800039a4:	557d                	li	a0,-1
    800039a6:	b7ed                	j	80003990 <dirlink+0x76>
    800039a8:	00004517          	auipc	a0,0x4
    800039ac:	c3050513          	add	a0,a0,-976 # 800075d8 <syscalls+0x210>
    800039b0:	e13fc0ef          	jal	800007c2 <panic>

00000000800039b4 <namei>:
    800039b4:	1101                	add	sp,sp,-32
    800039b6:	ec06                	sd	ra,24(sp)
    800039b8:	e822                	sd	s0,16(sp)
    800039ba:	1000                	add	s0,sp,32
    800039bc:	fe040613          	add	a2,s0,-32
    800039c0:	4581                	li	a1,0
    800039c2:	e21ff0ef          	jal	800037e2 <namex>
    800039c6:	60e2                	ld	ra,24(sp)
    800039c8:	6442                	ld	s0,16(sp)
    800039ca:	6105                	add	sp,sp,32
    800039cc:	8082                	ret

00000000800039ce <nameiparent>:
    800039ce:	1141                	add	sp,sp,-16
    800039d0:	e406                	sd	ra,8(sp)
    800039d2:	e022                	sd	s0,0(sp)
    800039d4:	0800                	add	s0,sp,16
    800039d6:	862e                	mv	a2,a1
    800039d8:	4585                	li	a1,1
    800039da:	e09ff0ef          	jal	800037e2 <namex>
    800039de:	60a2                	ld	ra,8(sp)
    800039e0:	6402                	ld	s0,0(sp)
    800039e2:	0141                	add	sp,sp,16
    800039e4:	8082                	ret

00000000800039e6 <write_head>:
    800039e6:	1101                	add	sp,sp,-32
    800039e8:	ec06                	sd	ra,24(sp)
    800039ea:	e822                	sd	s0,16(sp)
    800039ec:	e426                	sd	s1,8(sp)
    800039ee:	e04a                	sd	s2,0(sp)
    800039f0:	1000                	add	s0,sp,32
    800039f2:	0001c917          	auipc	s2,0x1c
    800039f6:	f3690913          	add	s2,s2,-202 # 8001f928 <log>
    800039fa:	01892583          	lw	a1,24(s2)
    800039fe:	02492503          	lw	a0,36(s2)
    80003a02:	8d0ff0ef          	jal	80002ad2 <bread>
    80003a06:	84aa                	mv	s1,a0
    80003a08:	02892683          	lw	a3,40(s2)
    80003a0c:	cd34                	sw	a3,88(a0)
    80003a0e:	02d05763          	blez	a3,80003a3c <write_head+0x56>
    80003a12:	0001c797          	auipc	a5,0x1c
    80003a16:	f4278793          	add	a5,a5,-190 # 8001f954 <log+0x2c>
    80003a1a:	05c50713          	add	a4,a0,92
    80003a1e:	36fd                	addw	a3,a3,-1
    80003a20:	1682                	sll	a3,a3,0x20
    80003a22:	9281                	srl	a3,a3,0x20
    80003a24:	068a                	sll	a3,a3,0x2
    80003a26:	0001c617          	auipc	a2,0x1c
    80003a2a:	f3260613          	add	a2,a2,-206 # 8001f958 <log+0x30>
    80003a2e:	96b2                	add	a3,a3,a2
    80003a30:	4390                	lw	a2,0(a5)
    80003a32:	c310                	sw	a2,0(a4)
    80003a34:	0791                	add	a5,a5,4
    80003a36:	0711                	add	a4,a4,4 # 43004 <_entry-0x7ffbcffc>
    80003a38:	fed79ce3          	bne	a5,a3,80003a30 <write_head+0x4a>
    80003a3c:	8526                	mv	a0,s1
    80003a3e:	97cff0ef          	jal	80002bba <bwrite>
    80003a42:	8526                	mv	a0,s1
    80003a44:	9a8ff0ef          	jal	80002bec <brelse>
    80003a48:	60e2                	ld	ra,24(sp)
    80003a4a:	6442                	ld	s0,16(sp)
    80003a4c:	64a2                	ld	s1,8(sp)
    80003a4e:	6902                	ld	s2,0(sp)
    80003a50:	6105                	add	sp,sp,32
    80003a52:	8082                	ret

0000000080003a54 <install_trans>:
    80003a54:	0001c797          	auipc	a5,0x1c
    80003a58:	ed478793          	add	a5,a5,-300 # 8001f928 <log>
    80003a5c:	579c                	lw	a5,40(a5)
    80003a5e:	0af05d63          	blez	a5,80003b18 <install_trans+0xc4>
    80003a62:	715d                	add	sp,sp,-80
    80003a64:	e486                	sd	ra,72(sp)
    80003a66:	e0a2                	sd	s0,64(sp)
    80003a68:	fc26                	sd	s1,56(sp)
    80003a6a:	f84a                	sd	s2,48(sp)
    80003a6c:	f44e                	sd	s3,40(sp)
    80003a6e:	f052                	sd	s4,32(sp)
    80003a70:	ec56                	sd	s5,24(sp)
    80003a72:	e85a                	sd	s6,16(sp)
    80003a74:	e45e                	sd	s7,8(sp)
    80003a76:	0880                	add	s0,sp,80
    80003a78:	8b2a                	mv	s6,a0
    80003a7a:	0001ca17          	auipc	s4,0x1c
    80003a7e:	edaa0a13          	add	s4,s4,-294 # 8001f954 <log+0x2c>
    80003a82:	4481                	li	s1,0
    80003a84:	00004b97          	auipc	s7,0x4
    80003a88:	b64b8b93          	add	s7,s7,-1180 # 800075e8 <syscalls+0x220>
    80003a8c:	0001c997          	auipc	s3,0x1c
    80003a90:	e9c98993          	add	s3,s3,-356 # 8001f928 <log>
    80003a94:	a03d                	j	80003ac2 <install_trans+0x6e>
    80003a96:	000a2603          	lw	a2,0(s4)
    80003a9a:	85a6                	mv	a1,s1
    80003a9c:	855e                	mv	a0,s7
    80003a9e:	a57fc0ef          	jal	800004f4 <printf>
    80003aa2:	a015                	j	80003ac6 <install_trans+0x72>
    80003aa4:	854a                	mv	a0,s2
    80003aa6:	a04ff0ef          	jal	80002caa <bunpin>
    80003aaa:	8556                	mv	a0,s5
    80003aac:	940ff0ef          	jal	80002bec <brelse>
    80003ab0:	854a                	mv	a0,s2
    80003ab2:	93aff0ef          	jal	80002bec <brelse>
    80003ab6:	2485                	addw	s1,s1,1
    80003ab8:	0a11                	add	s4,s4,4
    80003aba:	0289a783          	lw	a5,40(s3)
    80003abe:	04f4d263          	bge	s1,a5,80003b02 <install_trans+0xae>
    80003ac2:	fc0b1ae3          	bnez	s6,80003a96 <install_trans+0x42>
    80003ac6:	0189a583          	lw	a1,24(s3)
    80003aca:	9da5                	addw	a1,a1,s1
    80003acc:	2585                	addw	a1,a1,1
    80003ace:	0249a503          	lw	a0,36(s3)
    80003ad2:	800ff0ef          	jal	80002ad2 <bread>
    80003ad6:	8aaa                	mv	s5,a0
    80003ad8:	000a2583          	lw	a1,0(s4)
    80003adc:	0249a503          	lw	a0,36(s3)
    80003ae0:	ff3fe0ef          	jal	80002ad2 <bread>
    80003ae4:	892a                	mv	s2,a0
    80003ae6:	40000613          	li	a2,1024
    80003aea:	058a8593          	add	a1,s5,88
    80003aee:	05850513          	add	a0,a0,88
    80003af2:	a04fd0ef          	jal	80000cf6 <memmove>
    80003af6:	854a                	mv	a0,s2
    80003af8:	8c2ff0ef          	jal	80002bba <bwrite>
    80003afc:	fa0b17e3          	bnez	s6,80003aaa <install_trans+0x56>
    80003b00:	b755                	j	80003aa4 <install_trans+0x50>
    80003b02:	60a6                	ld	ra,72(sp)
    80003b04:	6406                	ld	s0,64(sp)
    80003b06:	74e2                	ld	s1,56(sp)
    80003b08:	7942                	ld	s2,48(sp)
    80003b0a:	79a2                	ld	s3,40(sp)
    80003b0c:	7a02                	ld	s4,32(sp)
    80003b0e:	6ae2                	ld	s5,24(sp)
    80003b10:	6b42                	ld	s6,16(sp)
    80003b12:	6ba2                	ld	s7,8(sp)
    80003b14:	6161                	add	sp,sp,80
    80003b16:	8082                	ret
    80003b18:	8082                	ret

0000000080003b1a <initlog>:
    80003b1a:	7179                	add	sp,sp,-48
    80003b1c:	f406                	sd	ra,40(sp)
    80003b1e:	f022                	sd	s0,32(sp)
    80003b20:	ec26                	sd	s1,24(sp)
    80003b22:	e84a                	sd	s2,16(sp)
    80003b24:	e44e                	sd	s3,8(sp)
    80003b26:	1800                	add	s0,sp,48
    80003b28:	892a                	mv	s2,a0
    80003b2a:	89ae                	mv	s3,a1
    80003b2c:	0001c497          	auipc	s1,0x1c
    80003b30:	dfc48493          	add	s1,s1,-516 # 8001f928 <log>
    80003b34:	00004597          	auipc	a1,0x4
    80003b38:	ad458593          	add	a1,a1,-1324 # 80007608 <syscalls+0x240>
    80003b3c:	8526                	mv	a0,s1
    80003b3e:	ff9fc0ef          	jal	80000b36 <initlock>
    80003b42:	0149a583          	lw	a1,20(s3)
    80003b46:	cc8c                	sw	a1,24(s1)
    80003b48:	0324a223          	sw	s2,36(s1)
    80003b4c:	854a                	mv	a0,s2
    80003b4e:	f85fe0ef          	jal	80002ad2 <bread>
    80003b52:	4d3c                	lw	a5,88(a0)
    80003b54:	d49c                	sw	a5,40(s1)
    80003b56:	02f05563          	blez	a5,80003b80 <initlog+0x66>
    80003b5a:	05c50713          	add	a4,a0,92
    80003b5e:	0001c697          	auipc	a3,0x1c
    80003b62:	df668693          	add	a3,a3,-522 # 8001f954 <log+0x2c>
    80003b66:	37fd                	addw	a5,a5,-1
    80003b68:	1782                	sll	a5,a5,0x20
    80003b6a:	9381                	srl	a5,a5,0x20
    80003b6c:	078a                	sll	a5,a5,0x2
    80003b6e:	06050613          	add	a2,a0,96
    80003b72:	97b2                	add	a5,a5,a2
    80003b74:	4310                	lw	a2,0(a4)
    80003b76:	c290                	sw	a2,0(a3)
    80003b78:	0711                	add	a4,a4,4
    80003b7a:	0691                	add	a3,a3,4
    80003b7c:	fef71ce3          	bne	a4,a5,80003b74 <initlog+0x5a>
    80003b80:	86cff0ef          	jal	80002bec <brelse>
    80003b84:	4505                	li	a0,1
    80003b86:	ecfff0ef          	jal	80003a54 <install_trans>
    80003b8a:	0001c797          	auipc	a5,0x1c
    80003b8e:	dc07a323          	sw	zero,-570(a5) # 8001f950 <log+0x28>
    80003b92:	e55ff0ef          	jal	800039e6 <write_head>
    80003b96:	70a2                	ld	ra,40(sp)
    80003b98:	7402                	ld	s0,32(sp)
    80003b9a:	64e2                	ld	s1,24(sp)
    80003b9c:	6942                	ld	s2,16(sp)
    80003b9e:	69a2                	ld	s3,8(sp)
    80003ba0:	6145                	add	sp,sp,48
    80003ba2:	8082                	ret

0000000080003ba4 <begin_op>:
    80003ba4:	1101                	add	sp,sp,-32
    80003ba6:	ec06                	sd	ra,24(sp)
    80003ba8:	e822                	sd	s0,16(sp)
    80003baa:	e426                	sd	s1,8(sp)
    80003bac:	e04a                	sd	s2,0(sp)
    80003bae:	1000                	add	s0,sp,32
    80003bb0:	0001c517          	auipc	a0,0x1c
    80003bb4:	d7850513          	add	a0,a0,-648 # 8001f928 <log>
    80003bb8:	ffffc0ef          	jal	80000bb6 <acquire>
    80003bbc:	0001c497          	auipc	s1,0x1c
    80003bc0:	d6c48493          	add	s1,s1,-660 # 8001f928 <log>
    80003bc4:	4979                	li	s2,30
    80003bc6:	a029                	j	80003bd0 <begin_op+0x2c>
    80003bc8:	85a6                	mv	a1,s1
    80003bca:	8526                	mv	a0,s1
    80003bcc:	ac8fe0ef          	jal	80001e94 <sleep>
    80003bd0:	509c                	lw	a5,32(s1)
    80003bd2:	fbfd                	bnez	a5,80003bc8 <begin_op+0x24>
    80003bd4:	4cdc                	lw	a5,28(s1)
    80003bd6:	0017871b          	addw	a4,a5,1
    80003bda:	0007069b          	sext.w	a3,a4
    80003bde:	0027179b          	sllw	a5,a4,0x2
    80003be2:	9fb9                	addw	a5,a5,a4
    80003be4:	0017979b          	sllw	a5,a5,0x1
    80003be8:	5498                	lw	a4,40(s1)
    80003bea:	9fb9                	addw	a5,a5,a4
    80003bec:	00f95763          	bge	s2,a5,80003bfa <begin_op+0x56>
    80003bf0:	85a6                	mv	a1,s1
    80003bf2:	8526                	mv	a0,s1
    80003bf4:	aa0fe0ef          	jal	80001e94 <sleep>
    80003bf8:	bfe1                	j	80003bd0 <begin_op+0x2c>
    80003bfa:	0001c517          	auipc	a0,0x1c
    80003bfe:	d2e50513          	add	a0,a0,-722 # 8001f928 <log>
    80003c02:	cd54                	sw	a3,28(a0)
    80003c04:	84afd0ef          	jal	80000c4e <release>
    80003c08:	60e2                	ld	ra,24(sp)
    80003c0a:	6442                	ld	s0,16(sp)
    80003c0c:	64a2                	ld	s1,8(sp)
    80003c0e:	6902                	ld	s2,0(sp)
    80003c10:	6105                	add	sp,sp,32
    80003c12:	8082                	ret

0000000080003c14 <end_op>:
    80003c14:	7139                	add	sp,sp,-64
    80003c16:	fc06                	sd	ra,56(sp)
    80003c18:	f822                	sd	s0,48(sp)
    80003c1a:	f426                	sd	s1,40(sp)
    80003c1c:	f04a                	sd	s2,32(sp)
    80003c1e:	ec4e                	sd	s3,24(sp)
    80003c20:	e852                	sd	s4,16(sp)
    80003c22:	e456                	sd	s5,8(sp)
    80003c24:	0080                	add	s0,sp,64
    80003c26:	0001c917          	auipc	s2,0x1c
    80003c2a:	d0290913          	add	s2,s2,-766 # 8001f928 <log>
    80003c2e:	854a                	mv	a0,s2
    80003c30:	f87fc0ef          	jal	80000bb6 <acquire>
    80003c34:	01c92783          	lw	a5,28(s2)
    80003c38:	37fd                	addw	a5,a5,-1
    80003c3a:	0007849b          	sext.w	s1,a5
    80003c3e:	00f92e23          	sw	a5,28(s2)
    80003c42:	02092783          	lw	a5,32(s2)
    80003c46:	e3a1                	bnez	a5,80003c86 <end_op+0x72>
    80003c48:	e4a9                	bnez	s1,80003c92 <end_op+0x7e>
    80003c4a:	0001c917          	auipc	s2,0x1c
    80003c4e:	cde90913          	add	s2,s2,-802 # 8001f928 <log>
    80003c52:	4785                	li	a5,1
    80003c54:	02f92023          	sw	a5,32(s2)
    80003c58:	854a                	mv	a0,s2
    80003c5a:	ff5fc0ef          	jal	80000c4e <release>
    80003c5e:	02892783          	lw	a5,40(s2)
    80003c62:	04f04b63          	bgtz	a5,80003cb8 <end_op+0xa4>
    80003c66:	0001c497          	auipc	s1,0x1c
    80003c6a:	cc248493          	add	s1,s1,-830 # 8001f928 <log>
    80003c6e:	8526                	mv	a0,s1
    80003c70:	f47fc0ef          	jal	80000bb6 <acquire>
    80003c74:	0204a023          	sw	zero,32(s1)
    80003c78:	8526                	mv	a0,s1
    80003c7a:	a66fe0ef          	jal	80001ee0 <wakeup>
    80003c7e:	8526                	mv	a0,s1
    80003c80:	fcffc0ef          	jal	80000c4e <release>
    80003c84:	a00d                	j	80003ca6 <end_op+0x92>
    80003c86:	00004517          	auipc	a0,0x4
    80003c8a:	98a50513          	add	a0,a0,-1654 # 80007610 <syscalls+0x248>
    80003c8e:	b35fc0ef          	jal	800007c2 <panic>
    80003c92:	0001c497          	auipc	s1,0x1c
    80003c96:	c9648493          	add	s1,s1,-874 # 8001f928 <log>
    80003c9a:	8526                	mv	a0,s1
    80003c9c:	a44fe0ef          	jal	80001ee0 <wakeup>
    80003ca0:	8526                	mv	a0,s1
    80003ca2:	fadfc0ef          	jal	80000c4e <release>
    80003ca6:	70e2                	ld	ra,56(sp)
    80003ca8:	7442                	ld	s0,48(sp)
    80003caa:	74a2                	ld	s1,40(sp)
    80003cac:	7902                	ld	s2,32(sp)
    80003cae:	69e2                	ld	s3,24(sp)
    80003cb0:	6a42                	ld	s4,16(sp)
    80003cb2:	6aa2                	ld	s5,8(sp)
    80003cb4:	6121                	add	sp,sp,64
    80003cb6:	8082                	ret
    80003cb8:	0001ca17          	auipc	s4,0x1c
    80003cbc:	c9ca0a13          	add	s4,s4,-868 # 8001f954 <log+0x2c>
    80003cc0:	0001c917          	auipc	s2,0x1c
    80003cc4:	c6890913          	add	s2,s2,-920 # 8001f928 <log>
    80003cc8:	01892583          	lw	a1,24(s2)
    80003ccc:	9da5                	addw	a1,a1,s1
    80003cce:	2585                	addw	a1,a1,1
    80003cd0:	02492503          	lw	a0,36(s2)
    80003cd4:	dfffe0ef          	jal	80002ad2 <bread>
    80003cd8:	89aa                	mv	s3,a0
    80003cda:	000a2583          	lw	a1,0(s4)
    80003cde:	02492503          	lw	a0,36(s2)
    80003ce2:	df1fe0ef          	jal	80002ad2 <bread>
    80003ce6:	8aaa                	mv	s5,a0
    80003ce8:	40000613          	li	a2,1024
    80003cec:	05850593          	add	a1,a0,88
    80003cf0:	05898513          	add	a0,s3,88
    80003cf4:	802fd0ef          	jal	80000cf6 <memmove>
    80003cf8:	854e                	mv	a0,s3
    80003cfa:	ec1fe0ef          	jal	80002bba <bwrite>
    80003cfe:	8556                	mv	a0,s5
    80003d00:	eedfe0ef          	jal	80002bec <brelse>
    80003d04:	854e                	mv	a0,s3
    80003d06:	ee7fe0ef          	jal	80002bec <brelse>
    80003d0a:	2485                	addw	s1,s1,1
    80003d0c:	0a11                	add	s4,s4,4
    80003d0e:	02892783          	lw	a5,40(s2)
    80003d12:	faf4cbe3          	blt	s1,a5,80003cc8 <end_op+0xb4>
    80003d16:	cd1ff0ef          	jal	800039e6 <write_head>
    80003d1a:	4501                	li	a0,0
    80003d1c:	d39ff0ef          	jal	80003a54 <install_trans>
    80003d20:	0001c797          	auipc	a5,0x1c
    80003d24:	c207a823          	sw	zero,-976(a5) # 8001f950 <log+0x28>
    80003d28:	cbfff0ef          	jal	800039e6 <write_head>
    80003d2c:	bf2d                	j	80003c66 <end_op+0x52>

0000000080003d2e <log_write>:
    80003d2e:	1101                	add	sp,sp,-32
    80003d30:	ec06                	sd	ra,24(sp)
    80003d32:	e822                	sd	s0,16(sp)
    80003d34:	e426                	sd	s1,8(sp)
    80003d36:	e04a                	sd	s2,0(sp)
    80003d38:	1000                	add	s0,sp,32
    80003d3a:	892a                	mv	s2,a0
    80003d3c:	0001c497          	auipc	s1,0x1c
    80003d40:	bec48493          	add	s1,s1,-1044 # 8001f928 <log>
    80003d44:	8526                	mv	a0,s1
    80003d46:	e71fc0ef          	jal	80000bb6 <acquire>
    80003d4a:	5490                	lw	a2,40(s1)
    80003d4c:	47f5                	li	a5,29
    80003d4e:	06c7c463          	blt	a5,a2,80003db6 <log_write+0x88>
    80003d52:	0001c797          	auipc	a5,0x1c
    80003d56:	bd678793          	add	a5,a5,-1066 # 8001f928 <log>
    80003d5a:	4fdc                	lw	a5,28(a5)
    80003d5c:	06f05363          	blez	a5,80003dc2 <log_write+0x94>
    80003d60:	06c05763          	blez	a2,80003dce <log_write+0xa0>
    80003d64:	00c92583          	lw	a1,12(s2)
    80003d68:	0001c797          	auipc	a5,0x1c
    80003d6c:	bc078793          	add	a5,a5,-1088 # 8001f928 <log>
    80003d70:	57dc                	lw	a5,44(a5)
    80003d72:	06b78663          	beq	a5,a1,80003dde <log_write+0xb0>
    80003d76:	0001c717          	auipc	a4,0x1c
    80003d7a:	be270713          	add	a4,a4,-1054 # 8001f958 <log+0x30>
    80003d7e:	4781                	li	a5,0
    80003d80:	2785                	addw	a5,a5,1
    80003d82:	06f60063          	beq	a2,a5,80003de2 <log_write+0xb4>
    80003d86:	4314                	lw	a3,0(a4)
    80003d88:	0711                	add	a4,a4,4
    80003d8a:	feb69be3          	bne	a3,a1,80003d80 <log_write+0x52>
    80003d8e:	07a1                	add	a5,a5,8
    80003d90:	078a                	sll	a5,a5,0x2
    80003d92:	0001c717          	auipc	a4,0x1c
    80003d96:	b9670713          	add	a4,a4,-1130 # 8001f928 <log>
    80003d9a:	97ba                	add	a5,a5,a4
    80003d9c:	c7cc                	sw	a1,12(a5)
    80003d9e:	0001c517          	auipc	a0,0x1c
    80003da2:	b8a50513          	add	a0,a0,-1142 # 8001f928 <log>
    80003da6:	ea9fc0ef          	jal	80000c4e <release>
    80003daa:	60e2                	ld	ra,24(sp)
    80003dac:	6442                	ld	s0,16(sp)
    80003dae:	64a2                	ld	s1,8(sp)
    80003db0:	6902                	ld	s2,0(sp)
    80003db2:	6105                	add	sp,sp,32
    80003db4:	8082                	ret
    80003db6:	00004517          	auipc	a0,0x4
    80003dba:	86a50513          	add	a0,a0,-1942 # 80007620 <syscalls+0x258>
    80003dbe:	a05fc0ef          	jal	800007c2 <panic>
    80003dc2:	00004517          	auipc	a0,0x4
    80003dc6:	87650513          	add	a0,a0,-1930 # 80007638 <syscalls+0x270>
    80003dca:	9f9fc0ef          	jal	800007c2 <panic>
    80003dce:	00c92783          	lw	a5,12(s2)
    80003dd2:	0001c717          	auipc	a4,0x1c
    80003dd6:	b8f72123          	sw	a5,-1150(a4) # 8001f954 <log+0x2c>
    80003dda:	f271                	bnez	a2,80003d9e <log_write+0x70>
    80003ddc:	a829                	j	80003df6 <log_write+0xc8>
    80003dde:	4781                	li	a5,0
    80003de0:	b77d                	j	80003d8e <log_write+0x60>
    80003de2:	0621                	add	a2,a2,8
    80003de4:	060a                	sll	a2,a2,0x2
    80003de6:	0001c797          	auipc	a5,0x1c
    80003dea:	b4278793          	add	a5,a5,-1214 # 8001f928 <log>
    80003dee:	963e                	add	a2,a2,a5
    80003df0:	00c92783          	lw	a5,12(s2)
    80003df4:	c65c                	sw	a5,12(a2)
    80003df6:	854a                	mv	a0,s2
    80003df8:	e7ffe0ef          	jal	80002c76 <bpin>
    80003dfc:	0001c717          	auipc	a4,0x1c
    80003e00:	b2c70713          	add	a4,a4,-1236 # 8001f928 <log>
    80003e04:	571c                	lw	a5,40(a4)
    80003e06:	2785                	addw	a5,a5,1
    80003e08:	d71c                	sw	a5,40(a4)
    80003e0a:	bf51                	j	80003d9e <log_write+0x70>

0000000080003e0c <initsleeplock>:
    80003e0c:	1101                	add	sp,sp,-32
    80003e0e:	ec06                	sd	ra,24(sp)
    80003e10:	e822                	sd	s0,16(sp)
    80003e12:	e426                	sd	s1,8(sp)
    80003e14:	e04a                	sd	s2,0(sp)
    80003e16:	1000                	add	s0,sp,32
    80003e18:	84aa                	mv	s1,a0
    80003e1a:	892e                	mv	s2,a1
    80003e1c:	00004597          	auipc	a1,0x4
    80003e20:	83c58593          	add	a1,a1,-1988 # 80007658 <syscalls+0x290>
    80003e24:	0521                	add	a0,a0,8
    80003e26:	d11fc0ef          	jal	80000b36 <initlock>
    80003e2a:	0324b023          	sd	s2,32(s1)
    80003e2e:	0004a023          	sw	zero,0(s1)
    80003e32:	0204a423          	sw	zero,40(s1)
    80003e36:	60e2                	ld	ra,24(sp)
    80003e38:	6442                	ld	s0,16(sp)
    80003e3a:	64a2                	ld	s1,8(sp)
    80003e3c:	6902                	ld	s2,0(sp)
    80003e3e:	6105                	add	sp,sp,32
    80003e40:	8082                	ret

0000000080003e42 <acquiresleep>:
    80003e42:	1101                	add	sp,sp,-32
    80003e44:	ec06                	sd	ra,24(sp)
    80003e46:	e822                	sd	s0,16(sp)
    80003e48:	e426                	sd	s1,8(sp)
    80003e4a:	e04a                	sd	s2,0(sp)
    80003e4c:	1000                	add	s0,sp,32
    80003e4e:	84aa                	mv	s1,a0
    80003e50:	00850913          	add	s2,a0,8
    80003e54:	854a                	mv	a0,s2
    80003e56:	d61fc0ef          	jal	80000bb6 <acquire>
    80003e5a:	409c                	lw	a5,0(s1)
    80003e5c:	c799                	beqz	a5,80003e6a <acquiresleep+0x28>
    80003e5e:	85ca                	mv	a1,s2
    80003e60:	8526                	mv	a0,s1
    80003e62:	832fe0ef          	jal	80001e94 <sleep>
    80003e66:	409c                	lw	a5,0(s1)
    80003e68:	fbfd                	bnez	a5,80003e5e <acquiresleep+0x1c>
    80003e6a:	4785                	li	a5,1
    80003e6c:	c09c                	sw	a5,0(s1)
    80003e6e:	a1ffd0ef          	jal	8000188c <myproc>
    80003e72:	591c                	lw	a5,48(a0)
    80003e74:	d49c                	sw	a5,40(s1)
    80003e76:	854a                	mv	a0,s2
    80003e78:	dd7fc0ef          	jal	80000c4e <release>
    80003e7c:	60e2                	ld	ra,24(sp)
    80003e7e:	6442                	ld	s0,16(sp)
    80003e80:	64a2                	ld	s1,8(sp)
    80003e82:	6902                	ld	s2,0(sp)
    80003e84:	6105                	add	sp,sp,32
    80003e86:	8082                	ret

0000000080003e88 <releasesleep>:
    80003e88:	1101                	add	sp,sp,-32
    80003e8a:	ec06                	sd	ra,24(sp)
    80003e8c:	e822                	sd	s0,16(sp)
    80003e8e:	e426                	sd	s1,8(sp)
    80003e90:	e04a                	sd	s2,0(sp)
    80003e92:	1000                	add	s0,sp,32
    80003e94:	84aa                	mv	s1,a0
    80003e96:	00850913          	add	s2,a0,8
    80003e9a:	854a                	mv	a0,s2
    80003e9c:	d1bfc0ef          	jal	80000bb6 <acquire>
    80003ea0:	0004a023          	sw	zero,0(s1)
    80003ea4:	0204a423          	sw	zero,40(s1)
    80003ea8:	8526                	mv	a0,s1
    80003eaa:	836fe0ef          	jal	80001ee0 <wakeup>
    80003eae:	854a                	mv	a0,s2
    80003eb0:	d9ffc0ef          	jal	80000c4e <release>
    80003eb4:	60e2                	ld	ra,24(sp)
    80003eb6:	6442                	ld	s0,16(sp)
    80003eb8:	64a2                	ld	s1,8(sp)
    80003eba:	6902                	ld	s2,0(sp)
    80003ebc:	6105                	add	sp,sp,32
    80003ebe:	8082                	ret

0000000080003ec0 <holdingsleep>:
    80003ec0:	7179                	add	sp,sp,-48
    80003ec2:	f406                	sd	ra,40(sp)
    80003ec4:	f022                	sd	s0,32(sp)
    80003ec6:	ec26                	sd	s1,24(sp)
    80003ec8:	e84a                	sd	s2,16(sp)
    80003eca:	e44e                	sd	s3,8(sp)
    80003ecc:	1800                	add	s0,sp,48
    80003ece:	84aa                	mv	s1,a0
    80003ed0:	00850913          	add	s2,a0,8
    80003ed4:	854a                	mv	a0,s2
    80003ed6:	ce1fc0ef          	jal	80000bb6 <acquire>
    80003eda:	409c                	lw	a5,0(s1)
    80003edc:	ef89                	bnez	a5,80003ef6 <holdingsleep+0x36>
    80003ede:	4481                	li	s1,0
    80003ee0:	854a                	mv	a0,s2
    80003ee2:	d6dfc0ef          	jal	80000c4e <release>
    80003ee6:	8526                	mv	a0,s1
    80003ee8:	70a2                	ld	ra,40(sp)
    80003eea:	7402                	ld	s0,32(sp)
    80003eec:	64e2                	ld	s1,24(sp)
    80003eee:	6942                	ld	s2,16(sp)
    80003ef0:	69a2                	ld	s3,8(sp)
    80003ef2:	6145                	add	sp,sp,48
    80003ef4:	8082                	ret
    80003ef6:	0284a983          	lw	s3,40(s1)
    80003efa:	993fd0ef          	jal	8000188c <myproc>
    80003efe:	5904                	lw	s1,48(a0)
    80003f00:	413484b3          	sub	s1,s1,s3
    80003f04:	0014b493          	seqz	s1,s1
    80003f08:	bfe1                	j	80003ee0 <holdingsleep+0x20>

0000000080003f0a <fileinit>:
    80003f0a:	1141                	add	sp,sp,-16
    80003f0c:	e406                	sd	ra,8(sp)
    80003f0e:	e022                	sd	s0,0(sp)
    80003f10:	0800                	add	s0,sp,16
    80003f12:	00003597          	auipc	a1,0x3
    80003f16:	75658593          	add	a1,a1,1878 # 80007668 <syscalls+0x2a0>
    80003f1a:	0001c517          	auipc	a0,0x1c
    80003f1e:	b5650513          	add	a0,a0,-1194 # 8001fa70 <ftable>
    80003f22:	c15fc0ef          	jal	80000b36 <initlock>
    80003f26:	60a2                	ld	ra,8(sp)
    80003f28:	6402                	ld	s0,0(sp)
    80003f2a:	0141                	add	sp,sp,16
    80003f2c:	8082                	ret

0000000080003f2e <filealloc>:
    80003f2e:	1101                	add	sp,sp,-32
    80003f30:	ec06                	sd	ra,24(sp)
    80003f32:	e822                	sd	s0,16(sp)
    80003f34:	e426                	sd	s1,8(sp)
    80003f36:	1000                	add	s0,sp,32
    80003f38:	0001c517          	auipc	a0,0x1c
    80003f3c:	b3850513          	add	a0,a0,-1224 # 8001fa70 <ftable>
    80003f40:	c77fc0ef          	jal	80000bb6 <acquire>
    80003f44:	0001c797          	auipc	a5,0x1c
    80003f48:	b2c78793          	add	a5,a5,-1236 # 8001fa70 <ftable>
    80003f4c:	4fdc                	lw	a5,28(a5)
    80003f4e:	c79d                	beqz	a5,80003f7c <filealloc+0x4e>
    80003f50:	0001c497          	auipc	s1,0x1c
    80003f54:	b6048493          	add	s1,s1,-1184 # 8001fab0 <ftable+0x40>
    80003f58:	0001d717          	auipc	a4,0x1d
    80003f5c:	ad070713          	add	a4,a4,-1328 # 80020a28 <disk>
    80003f60:	40dc                	lw	a5,4(s1)
    80003f62:	c38d                	beqz	a5,80003f84 <filealloc+0x56>
    80003f64:	02848493          	add	s1,s1,40
    80003f68:	fee49ce3          	bne	s1,a4,80003f60 <filealloc+0x32>
    80003f6c:	0001c517          	auipc	a0,0x1c
    80003f70:	b0450513          	add	a0,a0,-1276 # 8001fa70 <ftable>
    80003f74:	cdbfc0ef          	jal	80000c4e <release>
    80003f78:	4481                	li	s1,0
    80003f7a:	a829                	j	80003f94 <filealloc+0x66>
    80003f7c:	0001c497          	auipc	s1,0x1c
    80003f80:	b0c48493          	add	s1,s1,-1268 # 8001fa88 <ftable+0x18>
    80003f84:	4785                	li	a5,1
    80003f86:	c0dc                	sw	a5,4(s1)
    80003f88:	0001c517          	auipc	a0,0x1c
    80003f8c:	ae850513          	add	a0,a0,-1304 # 8001fa70 <ftable>
    80003f90:	cbffc0ef          	jal	80000c4e <release>
    80003f94:	8526                	mv	a0,s1
    80003f96:	60e2                	ld	ra,24(sp)
    80003f98:	6442                	ld	s0,16(sp)
    80003f9a:	64a2                	ld	s1,8(sp)
    80003f9c:	6105                	add	sp,sp,32
    80003f9e:	8082                	ret

0000000080003fa0 <filedup>:
    80003fa0:	1101                	add	sp,sp,-32
    80003fa2:	ec06                	sd	ra,24(sp)
    80003fa4:	e822                	sd	s0,16(sp)
    80003fa6:	e426                	sd	s1,8(sp)
    80003fa8:	1000                	add	s0,sp,32
    80003faa:	84aa                	mv	s1,a0
    80003fac:	0001c517          	auipc	a0,0x1c
    80003fb0:	ac450513          	add	a0,a0,-1340 # 8001fa70 <ftable>
    80003fb4:	c03fc0ef          	jal	80000bb6 <acquire>
    80003fb8:	40dc                	lw	a5,4(s1)
    80003fba:	02f05063          	blez	a5,80003fda <filedup+0x3a>
    80003fbe:	2785                	addw	a5,a5,1
    80003fc0:	c0dc                	sw	a5,4(s1)
    80003fc2:	0001c517          	auipc	a0,0x1c
    80003fc6:	aae50513          	add	a0,a0,-1362 # 8001fa70 <ftable>
    80003fca:	c85fc0ef          	jal	80000c4e <release>
    80003fce:	8526                	mv	a0,s1
    80003fd0:	60e2                	ld	ra,24(sp)
    80003fd2:	6442                	ld	s0,16(sp)
    80003fd4:	64a2                	ld	s1,8(sp)
    80003fd6:	6105                	add	sp,sp,32
    80003fd8:	8082                	ret
    80003fda:	00003517          	auipc	a0,0x3
    80003fde:	69650513          	add	a0,a0,1686 # 80007670 <syscalls+0x2a8>
    80003fe2:	fe0fc0ef          	jal	800007c2 <panic>

0000000080003fe6 <fileclose>:
    80003fe6:	7139                	add	sp,sp,-64
    80003fe8:	fc06                	sd	ra,56(sp)
    80003fea:	f822                	sd	s0,48(sp)
    80003fec:	f426                	sd	s1,40(sp)
    80003fee:	f04a                	sd	s2,32(sp)
    80003ff0:	ec4e                	sd	s3,24(sp)
    80003ff2:	e852                	sd	s4,16(sp)
    80003ff4:	e456                	sd	s5,8(sp)
    80003ff6:	0080                	add	s0,sp,64
    80003ff8:	84aa                	mv	s1,a0
    80003ffa:	0001c517          	auipc	a0,0x1c
    80003ffe:	a7650513          	add	a0,a0,-1418 # 8001fa70 <ftable>
    80004002:	bb5fc0ef          	jal	80000bb6 <acquire>
    80004006:	40dc                	lw	a5,4(s1)
    80004008:	04f05963          	blez	a5,8000405a <fileclose+0x74>
    8000400c:	37fd                	addw	a5,a5,-1
    8000400e:	0007871b          	sext.w	a4,a5
    80004012:	c0dc                	sw	a5,4(s1)
    80004014:	04e04963          	bgtz	a4,80004066 <fileclose+0x80>
    80004018:	0004a903          	lw	s2,0(s1)
    8000401c:	0094ca83          	lbu	s5,9(s1)
    80004020:	0104ba03          	ld	s4,16(s1)
    80004024:	0184b983          	ld	s3,24(s1)
    80004028:	0004a223          	sw	zero,4(s1)
    8000402c:	0004a023          	sw	zero,0(s1)
    80004030:	0001c517          	auipc	a0,0x1c
    80004034:	a4050513          	add	a0,a0,-1472 # 8001fa70 <ftable>
    80004038:	c17fc0ef          	jal	80000c4e <release>
    8000403c:	4785                	li	a5,1
    8000403e:	04f90363          	beq	s2,a5,80004084 <fileclose+0x9e>
    80004042:	3979                	addw	s2,s2,-2
    80004044:	4785                	li	a5,1
    80004046:	0327e663          	bltu	a5,s2,80004072 <fileclose+0x8c>
    8000404a:	b5bff0ef          	jal	80003ba4 <begin_op>
    8000404e:	854e                	mv	a0,s3
    80004050:	aeaff0ef          	jal	8000333a <iput>
    80004054:	bc1ff0ef          	jal	80003c14 <end_op>
    80004058:	a829                	j	80004072 <fileclose+0x8c>
    8000405a:	00003517          	auipc	a0,0x3
    8000405e:	61e50513          	add	a0,a0,1566 # 80007678 <syscalls+0x2b0>
    80004062:	f60fc0ef          	jal	800007c2 <panic>
    80004066:	0001c517          	auipc	a0,0x1c
    8000406a:	a0a50513          	add	a0,a0,-1526 # 8001fa70 <ftable>
    8000406e:	be1fc0ef          	jal	80000c4e <release>
    80004072:	70e2                	ld	ra,56(sp)
    80004074:	7442                	ld	s0,48(sp)
    80004076:	74a2                	ld	s1,40(sp)
    80004078:	7902                	ld	s2,32(sp)
    8000407a:	69e2                	ld	s3,24(sp)
    8000407c:	6a42                	ld	s4,16(sp)
    8000407e:	6aa2                	ld	s5,8(sp)
    80004080:	6121                	add	sp,sp,64
    80004082:	8082                	ret
    80004084:	85d6                	mv	a1,s5
    80004086:	8552                	mv	a0,s4
    80004088:	2e0000ef          	jal	80004368 <pipeclose>
    8000408c:	b7dd                	j	80004072 <fileclose+0x8c>

000000008000408e <filestat>:
    8000408e:	715d                	add	sp,sp,-80
    80004090:	e486                	sd	ra,72(sp)
    80004092:	e0a2                	sd	s0,64(sp)
    80004094:	fc26                	sd	s1,56(sp)
    80004096:	f84a                	sd	s2,48(sp)
    80004098:	f44e                	sd	s3,40(sp)
    8000409a:	0880                	add	s0,sp,80
    8000409c:	84aa                	mv	s1,a0
    8000409e:	89ae                	mv	s3,a1
    800040a0:	fecfd0ef          	jal	8000188c <myproc>
    800040a4:	409c                	lw	a5,0(s1)
    800040a6:	37f9                	addw	a5,a5,-2
    800040a8:	4705                	li	a4,1
    800040aa:	02f76f63          	bltu	a4,a5,800040e8 <filestat+0x5a>
    800040ae:	892a                	mv	s2,a0
    800040b0:	6c88                	ld	a0,24(s1)
    800040b2:	908ff0ef          	jal	800031ba <ilock>
    800040b6:	fb840593          	add	a1,s0,-72
    800040ba:	6c88                	ld	a0,24(s1)
    800040bc:	c64ff0ef          	jal	80003520 <stati>
    800040c0:	6c88                	ld	a0,24(s1)
    800040c2:	9a4ff0ef          	jal	80003266 <iunlock>
    800040c6:	46e1                	li	a3,24
    800040c8:	fb840613          	add	a2,s0,-72
    800040cc:	85ce                	mv	a1,s3
    800040ce:	05093503          	ld	a0,80(s2)
    800040d2:	d04fd0ef          	jal	800015d6 <copyout>
    800040d6:	41f5551b          	sraw	a0,a0,0x1f
    800040da:	60a6                	ld	ra,72(sp)
    800040dc:	6406                	ld	s0,64(sp)
    800040de:	74e2                	ld	s1,56(sp)
    800040e0:	7942                	ld	s2,48(sp)
    800040e2:	79a2                	ld	s3,40(sp)
    800040e4:	6161                	add	sp,sp,80
    800040e6:	8082                	ret
    800040e8:	557d                	li	a0,-1
    800040ea:	bfc5                	j	800040da <filestat+0x4c>

00000000800040ec <fileread>:
    800040ec:	7179                	add	sp,sp,-48
    800040ee:	f406                	sd	ra,40(sp)
    800040f0:	f022                	sd	s0,32(sp)
    800040f2:	ec26                	sd	s1,24(sp)
    800040f4:	e84a                	sd	s2,16(sp)
    800040f6:	e44e                	sd	s3,8(sp)
    800040f8:	1800                	add	s0,sp,48
    800040fa:	00854783          	lbu	a5,8(a0)
    800040fe:	cbc1                	beqz	a5,8000418e <fileread+0xa2>
    80004100:	89b2                	mv	s3,a2
    80004102:	892e                	mv	s2,a1
    80004104:	84aa                	mv	s1,a0
    80004106:	411c                	lw	a5,0(a0)
    80004108:	4705                	li	a4,1
    8000410a:	04e78363          	beq	a5,a4,80004150 <fileread+0x64>
    8000410e:	470d                	li	a4,3
    80004110:	04e78563          	beq	a5,a4,8000415a <fileread+0x6e>
    80004114:	4709                	li	a4,2
    80004116:	06e79663          	bne	a5,a4,80004182 <fileread+0x96>
    8000411a:	6d08                	ld	a0,24(a0)
    8000411c:	89eff0ef          	jal	800031ba <ilock>
    80004120:	874e                	mv	a4,s3
    80004122:	5094                	lw	a3,32(s1)
    80004124:	864a                	mv	a2,s2
    80004126:	4585                	li	a1,1
    80004128:	6c88                	ld	a0,24(s1)
    8000412a:	c20ff0ef          	jal	8000354a <readi>
    8000412e:	892a                	mv	s2,a0
    80004130:	00a05563          	blez	a0,8000413a <fileread+0x4e>
    80004134:	509c                	lw	a5,32(s1)
    80004136:	9fa9                	addw	a5,a5,a0
    80004138:	d09c                	sw	a5,32(s1)
    8000413a:	6c88                	ld	a0,24(s1)
    8000413c:	92aff0ef          	jal	80003266 <iunlock>
    80004140:	854a                	mv	a0,s2
    80004142:	70a2                	ld	ra,40(sp)
    80004144:	7402                	ld	s0,32(sp)
    80004146:	64e2                	ld	s1,24(sp)
    80004148:	6942                	ld	s2,16(sp)
    8000414a:	69a2                	ld	s3,8(sp)
    8000414c:	6145                	add	sp,sp,48
    8000414e:	8082                	ret
    80004150:	6908                	ld	a0,16(a0)
    80004152:	350000ef          	jal	800044a2 <piperead>
    80004156:	892a                	mv	s2,a0
    80004158:	b7e5                	j	80004140 <fileread+0x54>
    8000415a:	02451783          	lh	a5,36(a0)
    8000415e:	03079693          	sll	a3,a5,0x30
    80004162:	92c1                	srl	a3,a3,0x30
    80004164:	4725                	li	a4,9
    80004166:	02d76663          	bltu	a4,a3,80004192 <fileread+0xa6>
    8000416a:	0792                	sll	a5,a5,0x4
    8000416c:	0001c717          	auipc	a4,0x1c
    80004170:	86470713          	add	a4,a4,-1948 # 8001f9d0 <devsw>
    80004174:	97ba                	add	a5,a5,a4
    80004176:	639c                	ld	a5,0(a5)
    80004178:	cf99                	beqz	a5,80004196 <fileread+0xaa>
    8000417a:	4505                	li	a0,1
    8000417c:	9782                	jalr	a5
    8000417e:	892a                	mv	s2,a0
    80004180:	b7c1                	j	80004140 <fileread+0x54>
    80004182:	00003517          	auipc	a0,0x3
    80004186:	50650513          	add	a0,a0,1286 # 80007688 <syscalls+0x2c0>
    8000418a:	e38fc0ef          	jal	800007c2 <panic>
    8000418e:	597d                	li	s2,-1
    80004190:	bf45                	j	80004140 <fileread+0x54>
    80004192:	597d                	li	s2,-1
    80004194:	b775                	j	80004140 <fileread+0x54>
    80004196:	597d                	li	s2,-1
    80004198:	b765                	j	80004140 <fileread+0x54>

000000008000419a <filewrite>:
    8000419a:	715d                	add	sp,sp,-80
    8000419c:	e486                	sd	ra,72(sp)
    8000419e:	e0a2                	sd	s0,64(sp)
    800041a0:	fc26                	sd	s1,56(sp)
    800041a2:	f84a                	sd	s2,48(sp)
    800041a4:	f44e                	sd	s3,40(sp)
    800041a6:	f052                	sd	s4,32(sp)
    800041a8:	ec56                	sd	s5,24(sp)
    800041aa:	e85a                	sd	s6,16(sp)
    800041ac:	e45e                	sd	s7,8(sp)
    800041ae:	e062                	sd	s8,0(sp)
    800041b0:	0880                	add	s0,sp,80
    800041b2:	00954783          	lbu	a5,9(a0)
    800041b6:	0e078263          	beqz	a5,8000429a <filewrite+0x100>
    800041ba:	84aa                	mv	s1,a0
    800041bc:	8bae                	mv	s7,a1
    800041be:	8ab2                	mv	s5,a2
    800041c0:	411c                	lw	a5,0(a0)
    800041c2:	4705                	li	a4,1
    800041c4:	02e78263          	beq	a5,a4,800041e8 <filewrite+0x4e>
    800041c8:	470d                	li	a4,3
    800041ca:	02e78463          	beq	a5,a4,800041f2 <filewrite+0x58>
    800041ce:	4709                	li	a4,2
    800041d0:	0ae79f63          	bne	a5,a4,8000428e <filewrite+0xf4>
    800041d4:	08c05b63          	blez	a2,8000426a <filewrite+0xd0>
    800041d8:	4901                	li	s2,0
    800041da:	6b05                	lui	s6,0x1
    800041dc:	c00b0b13          	add	s6,s6,-1024 # c00 <_entry-0x7ffff400>
    800041e0:	6c05                	lui	s8,0x1
    800041e2:	c00c0c1b          	addw	s8,s8,-1024 # c00 <_entry-0x7ffff400>
    800041e6:	a895                	j	8000425a <filewrite+0xc0>
    800041e8:	6908                	ld	a0,16(a0)
    800041ea:	1d6000ef          	jal	800043c0 <pipewrite>
    800041ee:	8aaa                	mv	s5,a0
    800041f0:	a041                	j	80004270 <filewrite+0xd6>
    800041f2:	02451783          	lh	a5,36(a0)
    800041f6:	03079693          	sll	a3,a5,0x30
    800041fa:	92c1                	srl	a3,a3,0x30
    800041fc:	4725                	li	a4,9
    800041fe:	0ad76063          	bltu	a4,a3,8000429e <filewrite+0x104>
    80004202:	0792                	sll	a5,a5,0x4
    80004204:	0001b717          	auipc	a4,0x1b
    80004208:	7cc70713          	add	a4,a4,1996 # 8001f9d0 <devsw>
    8000420c:	97ba                	add	a5,a5,a4
    8000420e:	679c                	ld	a5,8(a5)
    80004210:	cbc9                	beqz	a5,800042a2 <filewrite+0x108>
    80004212:	4505                	li	a0,1
    80004214:	9782                	jalr	a5
    80004216:	8aaa                	mv	s5,a0
    80004218:	a8a1                	j	80004270 <filewrite+0xd6>
    8000421a:	00098a1b          	sext.w	s4,s3
    8000421e:	987ff0ef          	jal	80003ba4 <begin_op>
    80004222:	6c88                	ld	a0,24(s1)
    80004224:	f97fe0ef          	jal	800031ba <ilock>
    80004228:	8752                	mv	a4,s4
    8000422a:	5094                	lw	a3,32(s1)
    8000422c:	01790633          	add	a2,s2,s7
    80004230:	4585                	li	a1,1
    80004232:	6c88                	ld	a0,24(s1)
    80004234:	bfaff0ef          	jal	8000362e <writei>
    80004238:	89aa                	mv	s3,a0
    8000423a:	00a05563          	blez	a0,80004244 <filewrite+0xaa>
    8000423e:	509c                	lw	a5,32(s1)
    80004240:	9fa9                	addw	a5,a5,a0
    80004242:	d09c                	sw	a5,32(s1)
    80004244:	6c88                	ld	a0,24(s1)
    80004246:	820ff0ef          	jal	80003266 <iunlock>
    8000424a:	9cbff0ef          	jal	80003c14 <end_op>
    8000424e:	01499f63          	bne	s3,s4,8000426c <filewrite+0xd2>
    80004252:	012a093b          	addw	s2,s4,s2
    80004256:	01595b63          	bge	s2,s5,8000426c <filewrite+0xd2>
    8000425a:	412a87bb          	subw	a5,s5,s2
    8000425e:	89be                	mv	s3,a5
    80004260:	2781                	sext.w	a5,a5
    80004262:	fafb5ce3          	bge	s6,a5,8000421a <filewrite+0x80>
    80004266:	89e2                	mv	s3,s8
    80004268:	bf4d                	j	8000421a <filewrite+0x80>
    8000426a:	4901                	li	s2,0
    8000426c:	012a9f63          	bne	s5,s2,8000428a <filewrite+0xf0>
    80004270:	8556                	mv	a0,s5
    80004272:	60a6                	ld	ra,72(sp)
    80004274:	6406                	ld	s0,64(sp)
    80004276:	74e2                	ld	s1,56(sp)
    80004278:	7942                	ld	s2,48(sp)
    8000427a:	79a2                	ld	s3,40(sp)
    8000427c:	7a02                	ld	s4,32(sp)
    8000427e:	6ae2                	ld	s5,24(sp)
    80004280:	6b42                	ld	s6,16(sp)
    80004282:	6ba2                	ld	s7,8(sp)
    80004284:	6c02                	ld	s8,0(sp)
    80004286:	6161                	add	sp,sp,80
    80004288:	8082                	ret
    8000428a:	5afd                	li	s5,-1
    8000428c:	b7d5                	j	80004270 <filewrite+0xd6>
    8000428e:	00003517          	auipc	a0,0x3
    80004292:	40a50513          	add	a0,a0,1034 # 80007698 <syscalls+0x2d0>
    80004296:	d2cfc0ef          	jal	800007c2 <panic>
    8000429a:	5afd                	li	s5,-1
    8000429c:	bfd1                	j	80004270 <filewrite+0xd6>
    8000429e:	5afd                	li	s5,-1
    800042a0:	bfc1                	j	80004270 <filewrite+0xd6>
    800042a2:	5afd                	li	s5,-1
    800042a4:	b7f1                	j	80004270 <filewrite+0xd6>

00000000800042a6 <pipealloc>:
    800042a6:	7179                	add	sp,sp,-48
    800042a8:	f406                	sd	ra,40(sp)
    800042aa:	f022                	sd	s0,32(sp)
    800042ac:	ec26                	sd	s1,24(sp)
    800042ae:	e84a                	sd	s2,16(sp)
    800042b0:	e44e                	sd	s3,8(sp)
    800042b2:	e052                	sd	s4,0(sp)
    800042b4:	1800                	add	s0,sp,48
    800042b6:	84aa                	mv	s1,a0
    800042b8:	892e                	mv	s2,a1
    800042ba:	0005b023          	sd	zero,0(a1)
    800042be:	00053023          	sd	zero,0(a0)
    800042c2:	c6dff0ef          	jal	80003f2e <filealloc>
    800042c6:	e088                	sd	a0,0(s1)
    800042c8:	cd35                	beqz	a0,80004344 <pipealloc+0x9e>
    800042ca:	c65ff0ef          	jal	80003f2e <filealloc>
    800042ce:	00a93023          	sd	a0,0(s2)
    800042d2:	c52d                	beqz	a0,8000433c <pipealloc+0x96>
    800042d4:	813fc0ef          	jal	80000ae6 <kalloc>
    800042d8:	89aa                	mv	s3,a0
    800042da:	cd31                	beqz	a0,80004336 <pipealloc+0x90>
    800042dc:	4a05                	li	s4,1
    800042de:	23452023          	sw	s4,544(a0)
    800042e2:	23452223          	sw	s4,548(a0)
    800042e6:	20052e23          	sw	zero,540(a0)
    800042ea:	20052c23          	sw	zero,536(a0)
    800042ee:	00003597          	auipc	a1,0x3
    800042f2:	3ba58593          	add	a1,a1,954 # 800076a8 <syscalls+0x2e0>
    800042f6:	841fc0ef          	jal	80000b36 <initlock>
    800042fa:	609c                	ld	a5,0(s1)
    800042fc:	0147a023          	sw	s4,0(a5)
    80004300:	609c                	ld	a5,0(s1)
    80004302:	01478423          	sb	s4,8(a5)
    80004306:	609c                	ld	a5,0(s1)
    80004308:	000784a3          	sb	zero,9(a5)
    8000430c:	609c                	ld	a5,0(s1)
    8000430e:	0137b823          	sd	s3,16(a5)
    80004312:	00093783          	ld	a5,0(s2)
    80004316:	0147a023          	sw	s4,0(a5)
    8000431a:	00093783          	ld	a5,0(s2)
    8000431e:	00078423          	sb	zero,8(a5)
    80004322:	00093783          	ld	a5,0(s2)
    80004326:	014784a3          	sb	s4,9(a5)
    8000432a:	00093783          	ld	a5,0(s2)
    8000432e:	0137b823          	sd	s3,16(a5)
    80004332:	4501                	li	a0,0
    80004334:	a005                	j	80004354 <pipealloc+0xae>
    80004336:	6088                	ld	a0,0(s1)
    80004338:	e501                	bnez	a0,80004340 <pipealloc+0x9a>
    8000433a:	a029                	j	80004344 <pipealloc+0x9e>
    8000433c:	6088                	ld	a0,0(s1)
    8000433e:	c11d                	beqz	a0,80004364 <pipealloc+0xbe>
    80004340:	ca7ff0ef          	jal	80003fe6 <fileclose>
    80004344:	00093783          	ld	a5,0(s2)
    80004348:	557d                	li	a0,-1
    8000434a:	c789                	beqz	a5,80004354 <pipealloc+0xae>
    8000434c:	853e                	mv	a0,a5
    8000434e:	c99ff0ef          	jal	80003fe6 <fileclose>
    80004352:	557d                	li	a0,-1
    80004354:	70a2                	ld	ra,40(sp)
    80004356:	7402                	ld	s0,32(sp)
    80004358:	64e2                	ld	s1,24(sp)
    8000435a:	6942                	ld	s2,16(sp)
    8000435c:	69a2                	ld	s3,8(sp)
    8000435e:	6a02                	ld	s4,0(sp)
    80004360:	6145                	add	sp,sp,48
    80004362:	8082                	ret
    80004364:	557d                	li	a0,-1
    80004366:	b7fd                	j	80004354 <pipealloc+0xae>

0000000080004368 <pipeclose>:
    80004368:	1101                	add	sp,sp,-32
    8000436a:	ec06                	sd	ra,24(sp)
    8000436c:	e822                	sd	s0,16(sp)
    8000436e:	e426                	sd	s1,8(sp)
    80004370:	e04a                	sd	s2,0(sp)
    80004372:	1000                	add	s0,sp,32
    80004374:	84aa                	mv	s1,a0
    80004376:	892e                	mv	s2,a1
    80004378:	83ffc0ef          	jal	80000bb6 <acquire>
    8000437c:	02090763          	beqz	s2,800043aa <pipeclose+0x42>
    80004380:	2204a223          	sw	zero,548(s1)
    80004384:	21848513          	add	a0,s1,536
    80004388:	b59fd0ef          	jal	80001ee0 <wakeup>
    8000438c:	2204b783          	ld	a5,544(s1)
    80004390:	e785                	bnez	a5,800043b8 <pipeclose+0x50>
    80004392:	8526                	mv	a0,s1
    80004394:	8bbfc0ef          	jal	80000c4e <release>
    80004398:	8526                	mv	a0,s1
    8000439a:	e6afc0ef          	jal	80000a04 <kfree>
    8000439e:	60e2                	ld	ra,24(sp)
    800043a0:	6442                	ld	s0,16(sp)
    800043a2:	64a2                	ld	s1,8(sp)
    800043a4:	6902                	ld	s2,0(sp)
    800043a6:	6105                	add	sp,sp,32
    800043a8:	8082                	ret
    800043aa:	2204a023          	sw	zero,544(s1)
    800043ae:	21c48513          	add	a0,s1,540
    800043b2:	b2ffd0ef          	jal	80001ee0 <wakeup>
    800043b6:	bfd9                	j	8000438c <pipeclose+0x24>
    800043b8:	8526                	mv	a0,s1
    800043ba:	895fc0ef          	jal	80000c4e <release>
    800043be:	b7c5                	j	8000439e <pipeclose+0x36>

00000000800043c0 <pipewrite>:
    800043c0:	7159                	add	sp,sp,-112
    800043c2:	f486                	sd	ra,104(sp)
    800043c4:	f0a2                	sd	s0,96(sp)
    800043c6:	eca6                	sd	s1,88(sp)
    800043c8:	e8ca                	sd	s2,80(sp)
    800043ca:	e4ce                	sd	s3,72(sp)
    800043cc:	e0d2                	sd	s4,64(sp)
    800043ce:	fc56                	sd	s5,56(sp)
    800043d0:	f85a                	sd	s6,48(sp)
    800043d2:	f45e                	sd	s7,40(sp)
    800043d4:	f062                	sd	s8,32(sp)
    800043d6:	ec66                	sd	s9,24(sp)
    800043d8:	1880                	add	s0,sp,112
    800043da:	84aa                	mv	s1,a0
    800043dc:	8aae                	mv	s5,a1
    800043de:	8a32                	mv	s4,a2
    800043e0:	cacfd0ef          	jal	8000188c <myproc>
    800043e4:	89aa                	mv	s3,a0
    800043e6:	8526                	mv	a0,s1
    800043e8:	fcefc0ef          	jal	80000bb6 <acquire>
    800043ec:	0b405963          	blez	s4,8000449e <pipewrite+0xde>
    800043f0:	8ba6                	mv	s7,s1
    800043f2:	2204a783          	lw	a5,544(s1)
    800043f6:	cb81                	beqz	a5,80004406 <pipewrite+0x46>
    800043f8:	4901                	li	s2,0
    800043fa:	5b7d                	li	s6,-1
    800043fc:	21848c93          	add	s9,s1,536
    80004400:	21c48c13          	add	s8,s1,540
    80004404:	a8b1                	j	80004460 <pipewrite+0xa0>
    80004406:	8526                	mv	a0,s1
    80004408:	847fc0ef          	jal	80000c4e <release>
    8000440c:	597d                	li	s2,-1
    8000440e:	854a                	mv	a0,s2
    80004410:	70a6                	ld	ra,104(sp)
    80004412:	7406                	ld	s0,96(sp)
    80004414:	64e6                	ld	s1,88(sp)
    80004416:	6946                	ld	s2,80(sp)
    80004418:	69a6                	ld	s3,72(sp)
    8000441a:	6a06                	ld	s4,64(sp)
    8000441c:	7ae2                	ld	s5,56(sp)
    8000441e:	7b42                	ld	s6,48(sp)
    80004420:	7ba2                	ld	s7,40(sp)
    80004422:	7c02                	ld	s8,32(sp)
    80004424:	6ce2                	ld	s9,24(sp)
    80004426:	6165                	add	sp,sp,112
    80004428:	8082                	ret
    8000442a:	8566                	mv	a0,s9
    8000442c:	ab5fd0ef          	jal	80001ee0 <wakeup>
    80004430:	85de                	mv	a1,s7
    80004432:	8562                	mv	a0,s8
    80004434:	a61fd0ef          	jal	80001e94 <sleep>
    80004438:	a839                	j	80004456 <pipewrite+0x96>
    8000443a:	21c4a783          	lw	a5,540(s1)
    8000443e:	0017871b          	addw	a4,a5,1
    80004442:	20e4ae23          	sw	a4,540(s1)
    80004446:	1ff7f793          	and	a5,a5,511
    8000444a:	97a6                	add	a5,a5,s1
    8000444c:	f9f44703          	lbu	a4,-97(s0)
    80004450:	00e78c23          	sb	a4,24(a5)
    80004454:	2905                	addw	s2,s2,1
    80004456:	03495c63          	bge	s2,s4,8000448e <pipewrite+0xce>
    8000445a:	2204a783          	lw	a5,544(s1)
    8000445e:	d7c5                	beqz	a5,80004406 <pipewrite+0x46>
    80004460:	854e                	mv	a0,s3
    80004462:	c6dfd0ef          	jal	800020ce <killed>
    80004466:	f145                	bnez	a0,80004406 <pipewrite+0x46>
    80004468:	2184a783          	lw	a5,536(s1)
    8000446c:	21c4a703          	lw	a4,540(s1)
    80004470:	2007879b          	addw	a5,a5,512
    80004474:	faf70be3          	beq	a4,a5,8000442a <pipewrite+0x6a>
    80004478:	4685                	li	a3,1
    8000447a:	01590633          	add	a2,s2,s5
    8000447e:	f9f40593          	add	a1,s0,-97
    80004482:	0509b503          	ld	a0,80(s3)
    80004486:	a16fd0ef          	jal	8000169c <copyin>
    8000448a:	fb6518e3          	bne	a0,s6,8000443a <pipewrite+0x7a>
    8000448e:	21848513          	add	a0,s1,536
    80004492:	a4ffd0ef          	jal	80001ee0 <wakeup>
    80004496:	8526                	mv	a0,s1
    80004498:	fb6fc0ef          	jal	80000c4e <release>
    8000449c:	bf8d                	j	8000440e <pipewrite+0x4e>
    8000449e:	4901                	li	s2,0
    800044a0:	b7fd                	j	8000448e <pipewrite+0xce>

00000000800044a2 <piperead>:
    800044a2:	715d                	add	sp,sp,-80
    800044a4:	e486                	sd	ra,72(sp)
    800044a6:	e0a2                	sd	s0,64(sp)
    800044a8:	fc26                	sd	s1,56(sp)
    800044aa:	f84a                	sd	s2,48(sp)
    800044ac:	f44e                	sd	s3,40(sp)
    800044ae:	f052                	sd	s4,32(sp)
    800044b0:	ec56                	sd	s5,24(sp)
    800044b2:	e85a                	sd	s6,16(sp)
    800044b4:	0880                	add	s0,sp,80
    800044b6:	84aa                	mv	s1,a0
    800044b8:	89ae                	mv	s3,a1
    800044ba:	8ab2                	mv	s5,a2
    800044bc:	bd0fd0ef          	jal	8000188c <myproc>
    800044c0:	8a2a                	mv	s4,a0
    800044c2:	8526                	mv	a0,s1
    800044c4:	ef2fc0ef          	jal	80000bb6 <acquire>
    800044c8:	2184a703          	lw	a4,536(s1)
    800044cc:	21c4a783          	lw	a5,540(s1)
    800044d0:	06f71163          	bne	a4,a5,80004532 <piperead+0x90>
    800044d4:	8926                	mv	s2,s1
    800044d6:	2244a783          	lw	a5,548(s1)
    800044da:	c785                	beqz	a5,80004502 <piperead+0x60>
    800044dc:	21848b13          	add	s6,s1,536
    800044e0:	8552                	mv	a0,s4
    800044e2:	bedfd0ef          	jal	800020ce <killed>
    800044e6:	e129                	bnez	a0,80004528 <piperead+0x86>
    800044e8:	85ca                	mv	a1,s2
    800044ea:	855a                	mv	a0,s6
    800044ec:	9a9fd0ef          	jal	80001e94 <sleep>
    800044f0:	2184a703          	lw	a4,536(s1)
    800044f4:	21c4a783          	lw	a5,540(s1)
    800044f8:	02f71d63          	bne	a4,a5,80004532 <piperead+0x90>
    800044fc:	2244a783          	lw	a5,548(s1)
    80004500:	f3e5                	bnez	a5,800044e0 <piperead+0x3e>
    80004502:	4901                	li	s2,0
    80004504:	21c48513          	add	a0,s1,540
    80004508:	9d9fd0ef          	jal	80001ee0 <wakeup>
    8000450c:	8526                	mv	a0,s1
    8000450e:	f40fc0ef          	jal	80000c4e <release>
    80004512:	854a                	mv	a0,s2
    80004514:	60a6                	ld	ra,72(sp)
    80004516:	6406                	ld	s0,64(sp)
    80004518:	74e2                	ld	s1,56(sp)
    8000451a:	7942                	ld	s2,48(sp)
    8000451c:	79a2                	ld	s3,40(sp)
    8000451e:	7a02                	ld	s4,32(sp)
    80004520:	6ae2                	ld	s5,24(sp)
    80004522:	6b42                	ld	s6,16(sp)
    80004524:	6161                	add	sp,sp,80
    80004526:	8082                	ret
    80004528:	8526                	mv	a0,s1
    8000452a:	f24fc0ef          	jal	80000c4e <release>
    8000452e:	597d                	li	s2,-1
    80004530:	b7cd                	j	80004512 <piperead+0x70>
    80004532:	05505f63          	blez	s5,80004590 <piperead+0xee>
    80004536:	2184a783          	lw	a5,536(s1)
    8000453a:	21c4a703          	lw	a4,540(s1)
    8000453e:	04f70b63          	beq	a4,a5,80004594 <piperead+0xf2>
    80004542:	4901                	li	s2,0
    80004544:	5b7d                	li	s6,-1
    80004546:	1ff7f793          	and	a5,a5,511
    8000454a:	97a6                	add	a5,a5,s1
    8000454c:	0187c783          	lbu	a5,24(a5)
    80004550:	faf40fa3          	sb	a5,-65(s0)
    80004554:	4685                	li	a3,1
    80004556:	fbf40613          	add	a2,s0,-65
    8000455a:	85ce                	mv	a1,s3
    8000455c:	050a3503          	ld	a0,80(s4)
    80004560:	876fd0ef          	jal	800015d6 <copyout>
    80004564:	03650263          	beq	a0,s6,80004588 <piperead+0xe6>
    80004568:	2184a703          	lw	a4,536(s1)
    8000456c:	2705                	addw	a4,a4,1
    8000456e:	0007079b          	sext.w	a5,a4
    80004572:	20e4ac23          	sw	a4,536(s1)
    80004576:	2905                	addw	s2,s2,1
    80004578:	f92a86e3          	beq	s5,s2,80004504 <piperead+0x62>
    8000457c:	0985                	add	s3,s3,1
    8000457e:	21c4a703          	lw	a4,540(s1)
    80004582:	fcf712e3          	bne	a4,a5,80004546 <piperead+0xa4>
    80004586:	bfbd                	j	80004504 <piperead+0x62>
    80004588:	f6091ee3          	bnez	s2,80004504 <piperead+0x62>
    8000458c:	892a                	mv	s2,a0
    8000458e:	bf9d                	j	80004504 <piperead+0x62>
    80004590:	4901                	li	s2,0
    80004592:	bf8d                	j	80004504 <piperead+0x62>
    80004594:	4901                	li	s2,0
    80004596:	b7bd                	j	80004504 <piperead+0x62>

0000000080004598 <flags2perm>:
    80004598:	1141                	add	sp,sp,-16
    8000459a:	e422                	sd	s0,8(sp)
    8000459c:	0800                	add	s0,sp,16
    8000459e:	87aa                	mv	a5,a0
    800045a0:	8905                	and	a0,a0,1
    800045a2:	c111                	beqz	a0,800045a6 <flags2perm+0xe>
    800045a4:	4521                	li	a0,8
    800045a6:	8b89                	and	a5,a5,2
    800045a8:	c399                	beqz	a5,800045ae <flags2perm+0x16>
    800045aa:	00456513          	or	a0,a0,4
    800045ae:	6422                	ld	s0,8(sp)
    800045b0:	0141                	add	sp,sp,16
    800045b2:	8082                	ret

00000000800045b4 <kexec>:
    800045b4:	de010113          	add	sp,sp,-544
    800045b8:	20113c23          	sd	ra,536(sp)
    800045bc:	20813823          	sd	s0,528(sp)
    800045c0:	20913423          	sd	s1,520(sp)
    800045c4:	21213023          	sd	s2,512(sp)
    800045c8:	ffce                	sd	s3,504(sp)
    800045ca:	fbd2                	sd	s4,496(sp)
    800045cc:	f7d6                	sd	s5,488(sp)
    800045ce:	f3da                	sd	s6,480(sp)
    800045d0:	efde                	sd	s7,472(sp)
    800045d2:	ebe2                	sd	s8,464(sp)
    800045d4:	e7e6                	sd	s9,456(sp)
    800045d6:	e3ea                	sd	s10,448(sp)
    800045d8:	ff6e                	sd	s11,440(sp)
    800045da:	1400                	add	s0,sp,544
    800045dc:	892a                	mv	s2,a0
    800045de:	dea43823          	sd	a0,-528(s0)
    800045e2:	deb43c23          	sd	a1,-520(s0)
    800045e6:	aa6fd0ef          	jal	8000188c <myproc>
    800045ea:	84aa                	mv	s1,a0
    800045ec:	db8ff0ef          	jal	80003ba4 <begin_op>
    800045f0:	854a                	mv	a0,s2
    800045f2:	bc2ff0ef          	jal	800039b4 <namei>
    800045f6:	c13d                	beqz	a0,8000465c <kexec+0xa8>
    800045f8:	892a                	mv	s2,a0
    800045fa:	bc1fe0ef          	jal	800031ba <ilock>
    800045fe:	04000713          	li	a4,64
    80004602:	4681                	li	a3,0
    80004604:	e5040613          	add	a2,s0,-432
    80004608:	4581                	li	a1,0
    8000460a:	854a                	mv	a0,s2
    8000460c:	f3ffe0ef          	jal	8000354a <readi>
    80004610:	04000793          	li	a5,64
    80004614:	00f51a63          	bne	a0,a5,80004628 <kexec+0x74>
    80004618:	e5042703          	lw	a4,-432(s0)
    8000461c:	464c47b7          	lui	a5,0x464c4
    80004620:	57f78793          	add	a5,a5,1407 # 464c457f <_entry-0x39b3ba81>
    80004624:	04f70063          	beq	a4,a5,80004664 <kexec+0xb0>
    80004628:	854a                	mv	a0,s2
    8000462a:	d99fe0ef          	jal	800033c2 <iunlockput>
    8000462e:	de6ff0ef          	jal	80003c14 <end_op>
    80004632:	557d                	li	a0,-1
    80004634:	21813083          	ld	ra,536(sp)
    80004638:	21013403          	ld	s0,528(sp)
    8000463c:	20813483          	ld	s1,520(sp)
    80004640:	20013903          	ld	s2,512(sp)
    80004644:	79fe                	ld	s3,504(sp)
    80004646:	7a5e                	ld	s4,496(sp)
    80004648:	7abe                	ld	s5,488(sp)
    8000464a:	7b1e                	ld	s6,480(sp)
    8000464c:	6bfe                	ld	s7,472(sp)
    8000464e:	6c5e                	ld	s8,464(sp)
    80004650:	6cbe                	ld	s9,456(sp)
    80004652:	6d1e                	ld	s10,448(sp)
    80004654:	7dfa                	ld	s11,440(sp)
    80004656:	22010113          	add	sp,sp,544
    8000465a:	8082                	ret
    8000465c:	db8ff0ef          	jal	80003c14 <end_op>
    80004660:	557d                	li	a0,-1
    80004662:	bfc9                	j	80004634 <kexec+0x80>
    80004664:	8526                	mv	a0,s1
    80004666:	b2efd0ef          	jal	80001994 <proc_pagetable>
    8000466a:	e0a43423          	sd	a0,-504(s0)
    8000466e:	dd4d                	beqz	a0,80004628 <kexec+0x74>
    80004670:	e7042983          	lw	s3,-400(s0)
    80004674:	e8845783          	lhu	a5,-376(s0)
    80004678:	cfb9                	beqz	a5,800046d6 <kexec+0x122>
    8000467a:	4a01                	li	s4,0
    8000467c:	4b01                	li	s6,0
    8000467e:	6c05                	lui	s8,0x1
    80004680:	fffc0793          	add	a5,s8,-1 # fff <_entry-0x7ffff001>
    80004684:	def43423          	sd	a5,-536(s0)
    80004688:	7cfd                	lui	s9,0xfffff
    8000468a:	a411                	j	8000488e <kexec+0x2da>
    8000468c:	00003517          	auipc	a0,0x3
    80004690:	02450513          	add	a0,a0,36 # 800076b0 <syscalls+0x2e8>
    80004694:	92efc0ef          	jal	800007c2 <panic>
    80004698:	8756                	mv	a4,s5
    8000469a:	009d86bb          	addw	a3,s11,s1
    8000469e:	4581                	li	a1,0
    800046a0:	854a                	mv	a0,s2
    800046a2:	ea9fe0ef          	jal	8000354a <readi>
    800046a6:	2501                	sext.w	a0,a0
    800046a8:	18aa9863          	bne	s5,a0,80004838 <kexec+0x284>
    800046ac:	6785                	lui	a5,0x1
    800046ae:	9cbd                	addw	s1,s1,a5
    800046b0:	014c8a3b          	addw	s4,s9,s4
    800046b4:	1d74f463          	bgeu	s1,s7,8000487c <kexec+0x2c8>
    800046b8:	02049593          	sll	a1,s1,0x20
    800046bc:	9181                	srl	a1,a1,0x20
    800046be:	95ea                	add	a1,a1,s10
    800046c0:	e0843503          	ld	a0,-504(s0)
    800046c4:	909fc0ef          	jal	80000fcc <walkaddr>
    800046c8:	862a                	mv	a2,a0
    800046ca:	d169                	beqz	a0,8000468c <kexec+0xd8>
    800046cc:	8ae2                	mv	s5,s8
    800046ce:	fd8a75e3          	bgeu	s4,s8,80004698 <kexec+0xe4>
    800046d2:	8ad2                	mv	s5,s4
    800046d4:	b7d1                	j	80004698 <kexec+0xe4>
    800046d6:	4a01                	li	s4,0
    800046d8:	854a                	mv	a0,s2
    800046da:	ce9fe0ef          	jal	800033c2 <iunlockput>
    800046de:	d36ff0ef          	jal	80003c14 <end_op>
    800046e2:	9aafd0ef          	jal	8000188c <myproc>
    800046e6:	8aaa                	mv	s5,a0
    800046e8:	04853d03          	ld	s10,72(a0)
    800046ec:	6785                	lui	a5,0x1
    800046ee:	17fd                	add	a5,a5,-1 # fff <_entry-0x7ffff001>
    800046f0:	9a3e                	add	s4,s4,a5
    800046f2:	77fd                	lui	a5,0xfffff
    800046f4:	00fa77b3          	and	a5,s4,a5
    800046f8:	e0f43023          	sd	a5,-512(s0)
    800046fc:	4691                	li	a3,4
    800046fe:	6609                	lui	a2,0x2
    80004700:	963e                	add	a2,a2,a5
    80004702:	85be                	mv	a1,a5
    80004704:	e0843483          	ld	s1,-504(s0)
    80004708:	8526                	mv	a0,s1
    8000470a:	b89fc0ef          	jal	80001292 <uvmalloc>
    8000470e:	8b2a                	mv	s6,a0
    80004710:	4901                	li	s2,0
    80004712:	12050363          	beqz	a0,80004838 <kexec+0x284>
    80004716:	75f9                	lui	a1,0xffffe
    80004718:	95aa                	add	a1,a1,a0
    8000471a:	8526                	mv	a0,s1
    8000471c:	d3dfc0ef          	jal	80001458 <uvmclear>
    80004720:	7bfd                	lui	s7,0xfffff
    80004722:	9bda                	add	s7,s7,s6
    80004724:	df843783          	ld	a5,-520(s0)
    80004728:	6388                	ld	a0,0(a5)
    8000472a:	c135                	beqz	a0,8000478e <kexec+0x1da>
    8000472c:	e9040993          	add	s3,s0,-368
    80004730:	f9040c13          	add	s8,s0,-112
    80004734:	895a                	mv	s2,s6
    80004736:	4481                	li	s1,0
    80004738:	ef4fc0ef          	jal	80000e2c <strlen>
    8000473c:	2505                	addw	a0,a0,1
    8000473e:	40a90933          	sub	s2,s2,a0
    80004742:	ff097913          	and	s2,s2,-16
    80004746:	11796f63          	bltu	s2,s7,80004864 <kexec+0x2b0>
    8000474a:	df843c83          	ld	s9,-520(s0)
    8000474e:	000cba03          	ld	s4,0(s9) # fffffffffffff000 <end+0xffffffff7ffde498>
    80004752:	8552                	mv	a0,s4
    80004754:	ed8fc0ef          	jal	80000e2c <strlen>
    80004758:	0015069b          	addw	a3,a0,1
    8000475c:	8652                	mv	a2,s4
    8000475e:	85ca                	mv	a1,s2
    80004760:	e0843503          	ld	a0,-504(s0)
    80004764:	e73fc0ef          	jal	800015d6 <copyout>
    80004768:	10054263          	bltz	a0,8000486c <kexec+0x2b8>
    8000476c:	0129b023          	sd	s2,0(s3)
    80004770:	0485                	add	s1,s1,1
    80004772:	008c8793          	add	a5,s9,8
    80004776:	def43c23          	sd	a5,-520(s0)
    8000477a:	008cb503          	ld	a0,8(s9)
    8000477e:	c911                	beqz	a0,80004792 <kexec+0x1de>
    80004780:	09a1                	add	s3,s3,8
    80004782:	fb899be3          	bne	s3,s8,80004738 <kexec+0x184>
    80004786:	e1643023          	sd	s6,-512(s0)
    8000478a:	4901                	li	s2,0
    8000478c:	a075                	j	80004838 <kexec+0x284>
    8000478e:	895a                	mv	s2,s6
    80004790:	4481                	li	s1,0
    80004792:	00349793          	sll	a5,s1,0x3
    80004796:	f9040713          	add	a4,s0,-112
    8000479a:	97ba                	add	a5,a5,a4
    8000479c:	f007b023          	sd	zero,-256(a5) # ffffffffffffef00 <end+0xffffffff7ffde398>
    800047a0:	00148693          	add	a3,s1,1
    800047a4:	068e                	sll	a3,a3,0x3
    800047a6:	40d90933          	sub	s2,s2,a3
    800047aa:	ff097913          	and	s2,s2,-16
    800047ae:	01797663          	bgeu	s2,s7,800047ba <kexec+0x206>
    800047b2:	e1643023          	sd	s6,-512(s0)
    800047b6:	4901                	li	s2,0
    800047b8:	a041                	j	80004838 <kexec+0x284>
    800047ba:	e9040613          	add	a2,s0,-368
    800047be:	85ca                	mv	a1,s2
    800047c0:	e0843503          	ld	a0,-504(s0)
    800047c4:	e13fc0ef          	jal	800015d6 <copyout>
    800047c8:	0a054663          	bltz	a0,80004874 <kexec+0x2c0>
    800047cc:	058ab783          	ld	a5,88(s5)
    800047d0:	0727bc23          	sd	s2,120(a5)
    800047d4:	df043783          	ld	a5,-528(s0)
    800047d8:	0007c703          	lbu	a4,0(a5)
    800047dc:	cf11                	beqz	a4,800047f8 <kexec+0x244>
    800047de:	0785                	add	a5,a5,1
    800047e0:	02f00693          	li	a3,47
    800047e4:	a039                	j	800047f2 <kexec+0x23e>
    800047e6:	def43823          	sd	a5,-528(s0)
    800047ea:	0785                	add	a5,a5,1
    800047ec:	fff7c703          	lbu	a4,-1(a5)
    800047f0:	c701                	beqz	a4,800047f8 <kexec+0x244>
    800047f2:	fed71ce3          	bne	a4,a3,800047ea <kexec+0x236>
    800047f6:	bfc5                	j	800047e6 <kexec+0x232>
    800047f8:	4641                	li	a2,16
    800047fa:	df043583          	ld	a1,-528(s0)
    800047fe:	158a8513          	add	a0,s5,344
    80004802:	df8fc0ef          	jal	80000dfa <safestrcpy>
    80004806:	050ab503          	ld	a0,80(s5)
    8000480a:	e0843783          	ld	a5,-504(s0)
    8000480e:	04fab823          	sd	a5,80(s5)
    80004812:	056ab423          	sd	s6,72(s5)
    80004816:	058ab783          	ld	a5,88(s5)
    8000481a:	e6843703          	ld	a4,-408(s0)
    8000481e:	ef98                	sd	a4,24(a5)
    80004820:	058ab783          	ld	a5,88(s5)
    80004824:	0327b823          	sd	s2,48(a5)
    80004828:	85ea                	mv	a1,s10
    8000482a:	9eefd0ef          	jal	80001a18 <proc_freepagetable>
    8000482e:	0004851b          	sext.w	a0,s1
    80004832:	b509                	j	80004634 <kexec+0x80>
    80004834:	e1443023          	sd	s4,-512(s0)
    80004838:	e0043583          	ld	a1,-512(s0)
    8000483c:	e0843503          	ld	a0,-504(s0)
    80004840:	9d8fd0ef          	jal	80001a18 <proc_freepagetable>
    80004844:	de0912e3          	bnez	s2,80004628 <kexec+0x74>
    80004848:	557d                	li	a0,-1
    8000484a:	b3ed                	j	80004634 <kexec+0x80>
    8000484c:	e1443023          	sd	s4,-512(s0)
    80004850:	b7e5                	j	80004838 <kexec+0x284>
    80004852:	e1443023          	sd	s4,-512(s0)
    80004856:	b7cd                	j	80004838 <kexec+0x284>
    80004858:	e1443023          	sd	s4,-512(s0)
    8000485c:	bff1                	j	80004838 <kexec+0x284>
    8000485e:	e1443023          	sd	s4,-512(s0)
    80004862:	bfd9                	j	80004838 <kexec+0x284>
    80004864:	e1643023          	sd	s6,-512(s0)
    80004868:	4901                	li	s2,0
    8000486a:	b7f9                	j	80004838 <kexec+0x284>
    8000486c:	e1643023          	sd	s6,-512(s0)
    80004870:	4901                	li	s2,0
    80004872:	b7d9                	j	80004838 <kexec+0x284>
    80004874:	e1643023          	sd	s6,-512(s0)
    80004878:	4901                	li	s2,0
    8000487a:	bf7d                	j	80004838 <kexec+0x284>
    8000487c:	e0043a03          	ld	s4,-512(s0)
    80004880:	2b05                	addw	s6,s6,1
    80004882:	0389899b          	addw	s3,s3,56
    80004886:	e8845783          	lhu	a5,-376(s0)
    8000488a:	e4fb57e3          	bge	s6,a5,800046d8 <kexec+0x124>
    8000488e:	2981                	sext.w	s3,s3
    80004890:	03800713          	li	a4,56
    80004894:	86ce                	mv	a3,s3
    80004896:	e1840613          	add	a2,s0,-488
    8000489a:	4581                	li	a1,0
    8000489c:	854a                	mv	a0,s2
    8000489e:	cadfe0ef          	jal	8000354a <readi>
    800048a2:	03800793          	li	a5,56
    800048a6:	f8f517e3          	bne	a0,a5,80004834 <kexec+0x280>
    800048aa:	e1842783          	lw	a5,-488(s0)
    800048ae:	4705                	li	a4,1
    800048b0:	fce798e3          	bne	a5,a4,80004880 <kexec+0x2cc>
    800048b4:	e4043483          	ld	s1,-448(s0)
    800048b8:	e3843783          	ld	a5,-456(s0)
    800048bc:	f8f4e8e3          	bltu	s1,a5,8000484c <kexec+0x298>
    800048c0:	e2843783          	ld	a5,-472(s0)
    800048c4:	94be                	add	s1,s1,a5
    800048c6:	f8f4e6e3          	bltu	s1,a5,80004852 <kexec+0x29e>
    800048ca:	de843703          	ld	a4,-536(s0)
    800048ce:	8ff9                	and	a5,a5,a4
    800048d0:	f7c1                	bnez	a5,80004858 <kexec+0x2a4>
    800048d2:	e1c42503          	lw	a0,-484(s0)
    800048d6:	cc3ff0ef          	jal	80004598 <flags2perm>
    800048da:	86aa                	mv	a3,a0
    800048dc:	8626                	mv	a2,s1
    800048de:	85d2                	mv	a1,s4
    800048e0:	e0843503          	ld	a0,-504(s0)
    800048e4:	9affc0ef          	jal	80001292 <uvmalloc>
    800048e8:	e0a43023          	sd	a0,-512(s0)
    800048ec:	d92d                	beqz	a0,8000485e <kexec+0x2aa>
    800048ee:	e2843d03          	ld	s10,-472(s0)
    800048f2:	e2042d83          	lw	s11,-480(s0)
    800048f6:	e3842b83          	lw	s7,-456(s0)
    800048fa:	f80b81e3          	beqz	s7,8000487c <kexec+0x2c8>
    800048fe:	8a5e                	mv	s4,s7
    80004900:	4481                	li	s1,0
    80004902:	bb5d                	j	800046b8 <kexec+0x104>

0000000080004904 <argfd>:
    80004904:	7179                	add	sp,sp,-48
    80004906:	f406                	sd	ra,40(sp)
    80004908:	f022                	sd	s0,32(sp)
    8000490a:	ec26                	sd	s1,24(sp)
    8000490c:	e84a                	sd	s2,16(sp)
    8000490e:	1800                	add	s0,sp,48
    80004910:	892e                	mv	s2,a1
    80004912:	84b2                	mv	s1,a2
    80004914:	fdc40593          	add	a1,s0,-36
    80004918:	e7ffd0ef          	jal	80002796 <argint>
    8000491c:	fdc42703          	lw	a4,-36(s0)
    80004920:	47bd                	li	a5,15
    80004922:	02e7e963          	bltu	a5,a4,80004954 <argfd+0x50>
    80004926:	f67fc0ef          	jal	8000188c <myproc>
    8000492a:	fdc42703          	lw	a4,-36(s0)
    8000492e:	01a70793          	add	a5,a4,26
    80004932:	078e                	sll	a5,a5,0x3
    80004934:	953e                	add	a0,a0,a5
    80004936:	611c                	ld	a5,0(a0)
    80004938:	c385                	beqz	a5,80004958 <argfd+0x54>
    8000493a:	00090463          	beqz	s2,80004942 <argfd+0x3e>
    8000493e:	00e92023          	sw	a4,0(s2)
    80004942:	4501                	li	a0,0
    80004944:	c091                	beqz	s1,80004948 <argfd+0x44>
    80004946:	e09c                	sd	a5,0(s1)
    80004948:	70a2                	ld	ra,40(sp)
    8000494a:	7402                	ld	s0,32(sp)
    8000494c:	64e2                	ld	s1,24(sp)
    8000494e:	6942                	ld	s2,16(sp)
    80004950:	6145                	add	sp,sp,48
    80004952:	8082                	ret
    80004954:	557d                	li	a0,-1
    80004956:	bfcd                	j	80004948 <argfd+0x44>
    80004958:	557d                	li	a0,-1
    8000495a:	b7fd                	j	80004948 <argfd+0x44>

000000008000495c <fdalloc>:
    8000495c:	1101                	add	sp,sp,-32
    8000495e:	ec06                	sd	ra,24(sp)
    80004960:	e822                	sd	s0,16(sp)
    80004962:	e426                	sd	s1,8(sp)
    80004964:	1000                	add	s0,sp,32
    80004966:	84aa                	mv	s1,a0
    80004968:	f25fc0ef          	jal	8000188c <myproc>
    8000496c:	697c                	ld	a5,208(a0)
    8000496e:	c395                	beqz	a5,80004992 <fdalloc+0x36>
    80004970:	0d850713          	add	a4,a0,216
    80004974:	4785                	li	a5,1
    80004976:	4641                	li	a2,16
    80004978:	6314                	ld	a3,0(a4)
    8000497a:	ce89                	beqz	a3,80004994 <fdalloc+0x38>
    8000497c:	2785                	addw	a5,a5,1
    8000497e:	0721                	add	a4,a4,8
    80004980:	fec79ce3          	bne	a5,a2,80004978 <fdalloc+0x1c>
    80004984:	57fd                	li	a5,-1
    80004986:	853e                	mv	a0,a5
    80004988:	60e2                	ld	ra,24(sp)
    8000498a:	6442                	ld	s0,16(sp)
    8000498c:	64a2                	ld	s1,8(sp)
    8000498e:	6105                	add	sp,sp,32
    80004990:	8082                	ret
    80004992:	4781                	li	a5,0
    80004994:	01a78713          	add	a4,a5,26
    80004998:	070e                	sll	a4,a4,0x3
    8000499a:	953a                	add	a0,a0,a4
    8000499c:	e104                	sd	s1,0(a0)
    8000499e:	b7e5                	j	80004986 <fdalloc+0x2a>

00000000800049a0 <create>:
    800049a0:	715d                	add	sp,sp,-80
    800049a2:	e486                	sd	ra,72(sp)
    800049a4:	e0a2                	sd	s0,64(sp)
    800049a6:	fc26                	sd	s1,56(sp)
    800049a8:	f84a                	sd	s2,48(sp)
    800049aa:	f44e                	sd	s3,40(sp)
    800049ac:	f052                	sd	s4,32(sp)
    800049ae:	ec56                	sd	s5,24(sp)
    800049b0:	e85a                	sd	s6,16(sp)
    800049b2:	0880                	add	s0,sp,80
    800049b4:	89ae                	mv	s3,a1
    800049b6:	8b32                	mv	s6,a2
    800049b8:	8ab6                	mv	s5,a3
    800049ba:	fb040593          	add	a1,s0,-80
    800049be:	810ff0ef          	jal	800039ce <nameiparent>
    800049c2:	84aa                	mv	s1,a0
    800049c4:	10050963          	beqz	a0,80004ad6 <create+0x136>
    800049c8:	ff2fe0ef          	jal	800031ba <ilock>
    800049cc:	4601                	li	a2,0
    800049ce:	fb040593          	add	a1,s0,-80
    800049d2:	8526                	mv	a0,s1
    800049d4:	d73fe0ef          	jal	80003746 <dirlookup>
    800049d8:	892a                	mv	s2,a0
    800049da:	c139                	beqz	a0,80004a20 <create+0x80>
    800049dc:	8526                	mv	a0,s1
    800049de:	9e5fe0ef          	jal	800033c2 <iunlockput>
    800049e2:	854a                	mv	a0,s2
    800049e4:	fd6fe0ef          	jal	800031ba <ilock>
    800049e8:	2981                	sext.w	s3,s3
    800049ea:	4789                	li	a5,2
    800049ec:	02f99563          	bne	s3,a5,80004a16 <create+0x76>
    800049f0:	04495783          	lhu	a5,68(s2)
    800049f4:	37f9                	addw	a5,a5,-2
    800049f6:	17c2                	sll	a5,a5,0x30
    800049f8:	93c1                	srl	a5,a5,0x30
    800049fa:	4705                	li	a4,1
    800049fc:	00f76d63          	bltu	a4,a5,80004a16 <create+0x76>
    80004a00:	854a                	mv	a0,s2
    80004a02:	60a6                	ld	ra,72(sp)
    80004a04:	6406                	ld	s0,64(sp)
    80004a06:	74e2                	ld	s1,56(sp)
    80004a08:	7942                	ld	s2,48(sp)
    80004a0a:	79a2                	ld	s3,40(sp)
    80004a0c:	7a02                	ld	s4,32(sp)
    80004a0e:	6ae2                	ld	s5,24(sp)
    80004a10:	6b42                	ld	s6,16(sp)
    80004a12:	6161                	add	sp,sp,80
    80004a14:	8082                	ret
    80004a16:	854a                	mv	a0,s2
    80004a18:	9abfe0ef          	jal	800033c2 <iunlockput>
    80004a1c:	4901                	li	s2,0
    80004a1e:	b7cd                	j	80004a00 <create+0x60>
    80004a20:	85ce                	mv	a1,s3
    80004a22:	4088                	lw	a0,0(s1)
    80004a24:	e2afe0ef          	jal	8000304e <ialloc>
    80004a28:	8a2a                	mv	s4,a0
    80004a2a:	cd15                	beqz	a0,80004a66 <create+0xc6>
    80004a2c:	f8efe0ef          	jal	800031ba <ilock>
    80004a30:	056a1323          	sh	s6,70(s4)
    80004a34:	055a1423          	sh	s5,72(s4)
    80004a38:	4a85                	li	s5,1
    80004a3a:	055a1523          	sh	s5,74(s4)
    80004a3e:	8552                	mv	a0,s4
    80004a40:	ec6fe0ef          	jal	80003106 <iupdate>
    80004a44:	2981                	sext.w	s3,s3
    80004a46:	03598563          	beq	s3,s5,80004a70 <create+0xd0>
    80004a4a:	004a2603          	lw	a2,4(s4)
    80004a4e:	fb040593          	add	a1,s0,-80
    80004a52:	8526                	mv	a0,s1
    80004a54:	ec7fe0ef          	jal	8000391a <dirlink>
    80004a58:	06054363          	bltz	a0,80004abe <create+0x11e>
    80004a5c:	8526                	mv	a0,s1
    80004a5e:	965fe0ef          	jal	800033c2 <iunlockput>
    80004a62:	8952                	mv	s2,s4
    80004a64:	bf71                	j	80004a00 <create+0x60>
    80004a66:	8526                	mv	a0,s1
    80004a68:	95bfe0ef          	jal	800033c2 <iunlockput>
    80004a6c:	8952                	mv	s2,s4
    80004a6e:	bf49                	j	80004a00 <create+0x60>
    80004a70:	004a2603          	lw	a2,4(s4)
    80004a74:	00003597          	auipc	a1,0x3
    80004a78:	c5c58593          	add	a1,a1,-932 # 800076d0 <syscalls+0x308>
    80004a7c:	8552                	mv	a0,s4
    80004a7e:	e9dfe0ef          	jal	8000391a <dirlink>
    80004a82:	02054e63          	bltz	a0,80004abe <create+0x11e>
    80004a86:	40d0                	lw	a2,4(s1)
    80004a88:	00003597          	auipc	a1,0x3
    80004a8c:	c5058593          	add	a1,a1,-944 # 800076d8 <syscalls+0x310>
    80004a90:	8552                	mv	a0,s4
    80004a92:	e89fe0ef          	jal	8000391a <dirlink>
    80004a96:	02054463          	bltz	a0,80004abe <create+0x11e>
    80004a9a:	004a2603          	lw	a2,4(s4)
    80004a9e:	fb040593          	add	a1,s0,-80
    80004aa2:	8526                	mv	a0,s1
    80004aa4:	e77fe0ef          	jal	8000391a <dirlink>
    80004aa8:	00054b63          	bltz	a0,80004abe <create+0x11e>
    80004aac:	04a4d783          	lhu	a5,74(s1)
    80004ab0:	2785                	addw	a5,a5,1
    80004ab2:	04f49523          	sh	a5,74(s1)
    80004ab6:	8526                	mv	a0,s1
    80004ab8:	e4efe0ef          	jal	80003106 <iupdate>
    80004abc:	b745                	j	80004a5c <create+0xbc>
    80004abe:	040a1523          	sh	zero,74(s4)
    80004ac2:	8552                	mv	a0,s4
    80004ac4:	e42fe0ef          	jal	80003106 <iupdate>
    80004ac8:	8552                	mv	a0,s4
    80004aca:	8f9fe0ef          	jal	800033c2 <iunlockput>
    80004ace:	8526                	mv	a0,s1
    80004ad0:	8f3fe0ef          	jal	800033c2 <iunlockput>
    80004ad4:	b735                	j	80004a00 <create+0x60>
    80004ad6:	892a                	mv	s2,a0
    80004ad8:	b725                	j	80004a00 <create+0x60>

0000000080004ada <sys_dup>:
    80004ada:	7179                	add	sp,sp,-48
    80004adc:	f406                	sd	ra,40(sp)
    80004ade:	f022                	sd	s0,32(sp)
    80004ae0:	ec26                	sd	s1,24(sp)
    80004ae2:	1800                	add	s0,sp,48
    80004ae4:	fd840613          	add	a2,s0,-40
    80004ae8:	4581                	li	a1,0
    80004aea:	4501                	li	a0,0
    80004aec:	e19ff0ef          	jal	80004904 <argfd>
    80004af0:	57fd                	li	a5,-1
    80004af2:	00054f63          	bltz	a0,80004b10 <sys_dup+0x36>
    80004af6:	fd843503          	ld	a0,-40(s0)
    80004afa:	e63ff0ef          	jal	8000495c <fdalloc>
    80004afe:	84aa                	mv	s1,a0
    80004b00:	57fd                	li	a5,-1
    80004b02:	00054763          	bltz	a0,80004b10 <sys_dup+0x36>
    80004b06:	fd843503          	ld	a0,-40(s0)
    80004b0a:	c96ff0ef          	jal	80003fa0 <filedup>
    80004b0e:	87a6                	mv	a5,s1
    80004b10:	853e                	mv	a0,a5
    80004b12:	70a2                	ld	ra,40(sp)
    80004b14:	7402                	ld	s0,32(sp)
    80004b16:	64e2                	ld	s1,24(sp)
    80004b18:	6145                	add	sp,sp,48
    80004b1a:	8082                	ret

0000000080004b1c <sys_read>:
    80004b1c:	7179                	add	sp,sp,-48
    80004b1e:	f406                	sd	ra,40(sp)
    80004b20:	f022                	sd	s0,32(sp)
    80004b22:	1800                	add	s0,sp,48
    80004b24:	fd840593          	add	a1,s0,-40
    80004b28:	4505                	li	a0,1
    80004b2a:	c89fd0ef          	jal	800027b2 <argaddr>
    80004b2e:	fe440593          	add	a1,s0,-28
    80004b32:	4509                	li	a0,2
    80004b34:	c63fd0ef          	jal	80002796 <argint>
    80004b38:	fe840613          	add	a2,s0,-24
    80004b3c:	4581                	li	a1,0
    80004b3e:	4501                	li	a0,0
    80004b40:	dc5ff0ef          	jal	80004904 <argfd>
    80004b44:	57fd                	li	a5,-1
    80004b46:	00054b63          	bltz	a0,80004b5c <sys_read+0x40>
    80004b4a:	fe442603          	lw	a2,-28(s0)
    80004b4e:	fd843583          	ld	a1,-40(s0)
    80004b52:	fe843503          	ld	a0,-24(s0)
    80004b56:	d96ff0ef          	jal	800040ec <fileread>
    80004b5a:	87aa                	mv	a5,a0
    80004b5c:	853e                	mv	a0,a5
    80004b5e:	70a2                	ld	ra,40(sp)
    80004b60:	7402                	ld	s0,32(sp)
    80004b62:	6145                	add	sp,sp,48
    80004b64:	8082                	ret

0000000080004b66 <sys_write>:
    80004b66:	7179                	add	sp,sp,-48
    80004b68:	f406                	sd	ra,40(sp)
    80004b6a:	f022                	sd	s0,32(sp)
    80004b6c:	1800                	add	s0,sp,48
    80004b6e:	fd840593          	add	a1,s0,-40
    80004b72:	4505                	li	a0,1
    80004b74:	c3ffd0ef          	jal	800027b2 <argaddr>
    80004b78:	fe440593          	add	a1,s0,-28
    80004b7c:	4509                	li	a0,2
    80004b7e:	c19fd0ef          	jal	80002796 <argint>
    80004b82:	fe840613          	add	a2,s0,-24
    80004b86:	4581                	li	a1,0
    80004b88:	4501                	li	a0,0
    80004b8a:	d7bff0ef          	jal	80004904 <argfd>
    80004b8e:	57fd                	li	a5,-1
    80004b90:	00054b63          	bltz	a0,80004ba6 <sys_write+0x40>
    80004b94:	fe442603          	lw	a2,-28(s0)
    80004b98:	fd843583          	ld	a1,-40(s0)
    80004b9c:	fe843503          	ld	a0,-24(s0)
    80004ba0:	dfaff0ef          	jal	8000419a <filewrite>
    80004ba4:	87aa                	mv	a5,a0
    80004ba6:	853e                	mv	a0,a5
    80004ba8:	70a2                	ld	ra,40(sp)
    80004baa:	7402                	ld	s0,32(sp)
    80004bac:	6145                	add	sp,sp,48
    80004bae:	8082                	ret

0000000080004bb0 <sys_close>:
    80004bb0:	1101                	add	sp,sp,-32
    80004bb2:	ec06                	sd	ra,24(sp)
    80004bb4:	e822                	sd	s0,16(sp)
    80004bb6:	1000                	add	s0,sp,32
    80004bb8:	fe040613          	add	a2,s0,-32
    80004bbc:	fec40593          	add	a1,s0,-20
    80004bc0:	4501                	li	a0,0
    80004bc2:	d43ff0ef          	jal	80004904 <argfd>
    80004bc6:	57fd                	li	a5,-1
    80004bc8:	02054063          	bltz	a0,80004be8 <sys_close+0x38>
    80004bcc:	cc1fc0ef          	jal	8000188c <myproc>
    80004bd0:	fec42783          	lw	a5,-20(s0)
    80004bd4:	07e9                	add	a5,a5,26
    80004bd6:	078e                	sll	a5,a5,0x3
    80004bd8:	953e                	add	a0,a0,a5
    80004bda:	00053023          	sd	zero,0(a0)
    80004bde:	fe043503          	ld	a0,-32(s0)
    80004be2:	c04ff0ef          	jal	80003fe6 <fileclose>
    80004be6:	4781                	li	a5,0
    80004be8:	853e                	mv	a0,a5
    80004bea:	60e2                	ld	ra,24(sp)
    80004bec:	6442                	ld	s0,16(sp)
    80004bee:	6105                	add	sp,sp,32
    80004bf0:	8082                	ret

0000000080004bf2 <sys_fstat>:
    80004bf2:	1101                	add	sp,sp,-32
    80004bf4:	ec06                	sd	ra,24(sp)
    80004bf6:	e822                	sd	s0,16(sp)
    80004bf8:	1000                	add	s0,sp,32
    80004bfa:	fe040593          	add	a1,s0,-32
    80004bfe:	4505                	li	a0,1
    80004c00:	bb3fd0ef          	jal	800027b2 <argaddr>
    80004c04:	fe840613          	add	a2,s0,-24
    80004c08:	4581                	li	a1,0
    80004c0a:	4501                	li	a0,0
    80004c0c:	cf9ff0ef          	jal	80004904 <argfd>
    80004c10:	57fd                	li	a5,-1
    80004c12:	00054963          	bltz	a0,80004c24 <sys_fstat+0x32>
    80004c16:	fe043583          	ld	a1,-32(s0)
    80004c1a:	fe843503          	ld	a0,-24(s0)
    80004c1e:	c70ff0ef          	jal	8000408e <filestat>
    80004c22:	87aa                	mv	a5,a0
    80004c24:	853e                	mv	a0,a5
    80004c26:	60e2                	ld	ra,24(sp)
    80004c28:	6442                	ld	s0,16(sp)
    80004c2a:	6105                	add	sp,sp,32
    80004c2c:	8082                	ret

0000000080004c2e <sys_link>:
    80004c2e:	7169                	add	sp,sp,-304
    80004c30:	f606                	sd	ra,296(sp)
    80004c32:	f222                	sd	s0,288(sp)
    80004c34:	ee26                	sd	s1,280(sp)
    80004c36:	ea4a                	sd	s2,272(sp)
    80004c38:	1a00                	add	s0,sp,304
    80004c3a:	08000613          	li	a2,128
    80004c3e:	ed040593          	add	a1,s0,-304
    80004c42:	4501                	li	a0,0
    80004c44:	b8bfd0ef          	jal	800027ce <argstr>
    80004c48:	57fd                	li	a5,-1
    80004c4a:	0c054663          	bltz	a0,80004d16 <sys_link+0xe8>
    80004c4e:	08000613          	li	a2,128
    80004c52:	f5040593          	add	a1,s0,-176
    80004c56:	4505                	li	a0,1
    80004c58:	b77fd0ef          	jal	800027ce <argstr>
    80004c5c:	57fd                	li	a5,-1
    80004c5e:	0a054c63          	bltz	a0,80004d16 <sys_link+0xe8>
    80004c62:	f43fe0ef          	jal	80003ba4 <begin_op>
    80004c66:	ed040513          	add	a0,s0,-304
    80004c6a:	d4bfe0ef          	jal	800039b4 <namei>
    80004c6e:	84aa                	mv	s1,a0
    80004c70:	c525                	beqz	a0,80004cd8 <sys_link+0xaa>
    80004c72:	d48fe0ef          	jal	800031ba <ilock>
    80004c76:	04449703          	lh	a4,68(s1)
    80004c7a:	4785                	li	a5,1
    80004c7c:	06f70263          	beq	a4,a5,80004ce0 <sys_link+0xb2>
    80004c80:	04a4d783          	lhu	a5,74(s1)
    80004c84:	2785                	addw	a5,a5,1
    80004c86:	04f49523          	sh	a5,74(s1)
    80004c8a:	8526                	mv	a0,s1
    80004c8c:	c7afe0ef          	jal	80003106 <iupdate>
    80004c90:	8526                	mv	a0,s1
    80004c92:	dd4fe0ef          	jal	80003266 <iunlock>
    80004c96:	fd040593          	add	a1,s0,-48
    80004c9a:	f5040513          	add	a0,s0,-176
    80004c9e:	d31fe0ef          	jal	800039ce <nameiparent>
    80004ca2:	892a                	mv	s2,a0
    80004ca4:	c921                	beqz	a0,80004cf4 <sys_link+0xc6>
    80004ca6:	d14fe0ef          	jal	800031ba <ilock>
    80004caa:	00092703          	lw	a4,0(s2)
    80004cae:	409c                	lw	a5,0(s1)
    80004cb0:	02f71f63          	bne	a4,a5,80004cee <sys_link+0xc0>
    80004cb4:	40d0                	lw	a2,4(s1)
    80004cb6:	fd040593          	add	a1,s0,-48
    80004cba:	854a                	mv	a0,s2
    80004cbc:	c5ffe0ef          	jal	8000391a <dirlink>
    80004cc0:	02054763          	bltz	a0,80004cee <sys_link+0xc0>
    80004cc4:	854a                	mv	a0,s2
    80004cc6:	efcfe0ef          	jal	800033c2 <iunlockput>
    80004cca:	8526                	mv	a0,s1
    80004ccc:	e6efe0ef          	jal	8000333a <iput>
    80004cd0:	f45fe0ef          	jal	80003c14 <end_op>
    80004cd4:	4781                	li	a5,0
    80004cd6:	a081                	j	80004d16 <sys_link+0xe8>
    80004cd8:	f3dfe0ef          	jal	80003c14 <end_op>
    80004cdc:	57fd                	li	a5,-1
    80004cde:	a825                	j	80004d16 <sys_link+0xe8>
    80004ce0:	8526                	mv	a0,s1
    80004ce2:	ee0fe0ef          	jal	800033c2 <iunlockput>
    80004ce6:	f2ffe0ef          	jal	80003c14 <end_op>
    80004cea:	57fd                	li	a5,-1
    80004cec:	a02d                	j	80004d16 <sys_link+0xe8>
    80004cee:	854a                	mv	a0,s2
    80004cf0:	ed2fe0ef          	jal	800033c2 <iunlockput>
    80004cf4:	8526                	mv	a0,s1
    80004cf6:	cc4fe0ef          	jal	800031ba <ilock>
    80004cfa:	04a4d783          	lhu	a5,74(s1)
    80004cfe:	37fd                	addw	a5,a5,-1
    80004d00:	04f49523          	sh	a5,74(s1)
    80004d04:	8526                	mv	a0,s1
    80004d06:	c00fe0ef          	jal	80003106 <iupdate>
    80004d0a:	8526                	mv	a0,s1
    80004d0c:	eb6fe0ef          	jal	800033c2 <iunlockput>
    80004d10:	f05fe0ef          	jal	80003c14 <end_op>
    80004d14:	57fd                	li	a5,-1
    80004d16:	853e                	mv	a0,a5
    80004d18:	70b2                	ld	ra,296(sp)
    80004d1a:	7412                	ld	s0,288(sp)
    80004d1c:	64f2                	ld	s1,280(sp)
    80004d1e:	6952                	ld	s2,272(sp)
    80004d20:	6155                	add	sp,sp,304
    80004d22:	8082                	ret

0000000080004d24 <sys_unlink>:
    80004d24:	7151                	add	sp,sp,-240
    80004d26:	f586                	sd	ra,232(sp)
    80004d28:	f1a2                	sd	s0,224(sp)
    80004d2a:	eda6                	sd	s1,216(sp)
    80004d2c:	e9ca                	sd	s2,208(sp)
    80004d2e:	e5ce                	sd	s3,200(sp)
    80004d30:	1980                	add	s0,sp,240
    80004d32:	08000613          	li	a2,128
    80004d36:	f3040593          	add	a1,s0,-208
    80004d3a:	4501                	li	a0,0
    80004d3c:	a93fd0ef          	jal	800027ce <argstr>
    80004d40:	12054963          	bltz	a0,80004e72 <sys_unlink+0x14e>
    80004d44:	e61fe0ef          	jal	80003ba4 <begin_op>
    80004d48:	fb040593          	add	a1,s0,-80
    80004d4c:	f3040513          	add	a0,s0,-208
    80004d50:	c7ffe0ef          	jal	800039ce <nameiparent>
    80004d54:	89aa                	mv	s3,a0
    80004d56:	c54d                	beqz	a0,80004e00 <sys_unlink+0xdc>
    80004d58:	c62fe0ef          	jal	800031ba <ilock>
    80004d5c:	00003597          	auipc	a1,0x3
    80004d60:	97458593          	add	a1,a1,-1676 # 800076d0 <syscalls+0x308>
    80004d64:	fb040513          	add	a0,s0,-80
    80004d68:	9c9fe0ef          	jal	80003730 <namecmp>
    80004d6c:	10050863          	beqz	a0,80004e7c <sys_unlink+0x158>
    80004d70:	00003597          	auipc	a1,0x3
    80004d74:	96858593          	add	a1,a1,-1688 # 800076d8 <syscalls+0x310>
    80004d78:	fb040513          	add	a0,s0,-80
    80004d7c:	9b5fe0ef          	jal	80003730 <namecmp>
    80004d80:	0e050e63          	beqz	a0,80004e7c <sys_unlink+0x158>
    80004d84:	f2c40613          	add	a2,s0,-212
    80004d88:	fb040593          	add	a1,s0,-80
    80004d8c:	854e                	mv	a0,s3
    80004d8e:	9b9fe0ef          	jal	80003746 <dirlookup>
    80004d92:	84aa                	mv	s1,a0
    80004d94:	0e050463          	beqz	a0,80004e7c <sys_unlink+0x158>
    80004d98:	c22fe0ef          	jal	800031ba <ilock>
    80004d9c:	04a49783          	lh	a5,74(s1)
    80004da0:	06f05463          	blez	a5,80004e08 <sys_unlink+0xe4>
    80004da4:	04449703          	lh	a4,68(s1)
    80004da8:	4785                	li	a5,1
    80004daa:	06f70563          	beq	a4,a5,80004e14 <sys_unlink+0xf0>
    80004dae:	4641                	li	a2,16
    80004db0:	4581                	li	a1,0
    80004db2:	fc040513          	add	a0,s0,-64
    80004db6:	ed5fb0ef          	jal	80000c8a <memset>
    80004dba:	4741                	li	a4,16
    80004dbc:	f2c42683          	lw	a3,-212(s0)
    80004dc0:	fc040613          	add	a2,s0,-64
    80004dc4:	4581                	li	a1,0
    80004dc6:	854e                	mv	a0,s3
    80004dc8:	867fe0ef          	jal	8000362e <writei>
    80004dcc:	47c1                	li	a5,16
    80004dce:	08f51363          	bne	a0,a5,80004e54 <sys_unlink+0x130>
    80004dd2:	04449703          	lh	a4,68(s1)
    80004dd6:	4785                	li	a5,1
    80004dd8:	08f70463          	beq	a4,a5,80004e60 <sys_unlink+0x13c>
    80004ddc:	854e                	mv	a0,s3
    80004dde:	de4fe0ef          	jal	800033c2 <iunlockput>
    80004de2:	04a4d783          	lhu	a5,74(s1)
    80004de6:	37fd                	addw	a5,a5,-1
    80004de8:	04f49523          	sh	a5,74(s1)
    80004dec:	8526                	mv	a0,s1
    80004dee:	b18fe0ef          	jal	80003106 <iupdate>
    80004df2:	8526                	mv	a0,s1
    80004df4:	dcefe0ef          	jal	800033c2 <iunlockput>
    80004df8:	e1dfe0ef          	jal	80003c14 <end_op>
    80004dfc:	4501                	li	a0,0
    80004dfe:	a069                	j	80004e88 <sys_unlink+0x164>
    80004e00:	e15fe0ef          	jal	80003c14 <end_op>
    80004e04:	557d                	li	a0,-1
    80004e06:	a049                	j	80004e88 <sys_unlink+0x164>
    80004e08:	00003517          	auipc	a0,0x3
    80004e0c:	8d850513          	add	a0,a0,-1832 # 800076e0 <syscalls+0x318>
    80004e10:	9b3fb0ef          	jal	800007c2 <panic>
    80004e14:	44f8                	lw	a4,76(s1)
    80004e16:	02000793          	li	a5,32
    80004e1a:	f8e7fae3          	bgeu	a5,a4,80004dae <sys_unlink+0x8a>
    80004e1e:	02000913          	li	s2,32
    80004e22:	4741                	li	a4,16
    80004e24:	86ca                	mv	a3,s2
    80004e26:	f1840613          	add	a2,s0,-232
    80004e2a:	4581                	li	a1,0
    80004e2c:	8526                	mv	a0,s1
    80004e2e:	f1cfe0ef          	jal	8000354a <readi>
    80004e32:	47c1                	li	a5,16
    80004e34:	00f51a63          	bne	a0,a5,80004e48 <sys_unlink+0x124>
    80004e38:	f1845783          	lhu	a5,-232(s0)
    80004e3c:	ef8d                	bnez	a5,80004e76 <sys_unlink+0x152>
    80004e3e:	2941                	addw	s2,s2,16
    80004e40:	44fc                	lw	a5,76(s1)
    80004e42:	fef960e3          	bltu	s2,a5,80004e22 <sys_unlink+0xfe>
    80004e46:	b7a5                	j	80004dae <sys_unlink+0x8a>
    80004e48:	00003517          	auipc	a0,0x3
    80004e4c:	8b050513          	add	a0,a0,-1872 # 800076f8 <syscalls+0x330>
    80004e50:	973fb0ef          	jal	800007c2 <panic>
    80004e54:	00003517          	auipc	a0,0x3
    80004e58:	8bc50513          	add	a0,a0,-1860 # 80007710 <syscalls+0x348>
    80004e5c:	967fb0ef          	jal	800007c2 <panic>
    80004e60:	04a9d783          	lhu	a5,74(s3)
    80004e64:	37fd                	addw	a5,a5,-1
    80004e66:	04f99523          	sh	a5,74(s3)
    80004e6a:	854e                	mv	a0,s3
    80004e6c:	a9afe0ef          	jal	80003106 <iupdate>
    80004e70:	b7b5                	j	80004ddc <sys_unlink+0xb8>
    80004e72:	557d                	li	a0,-1
    80004e74:	a811                	j	80004e88 <sys_unlink+0x164>
    80004e76:	8526                	mv	a0,s1
    80004e78:	d4afe0ef          	jal	800033c2 <iunlockput>
    80004e7c:	854e                	mv	a0,s3
    80004e7e:	d44fe0ef          	jal	800033c2 <iunlockput>
    80004e82:	d93fe0ef          	jal	80003c14 <end_op>
    80004e86:	557d                	li	a0,-1
    80004e88:	70ae                	ld	ra,232(sp)
    80004e8a:	740e                	ld	s0,224(sp)
    80004e8c:	64ee                	ld	s1,216(sp)
    80004e8e:	694e                	ld	s2,208(sp)
    80004e90:	69ae                	ld	s3,200(sp)
    80004e92:	616d                	add	sp,sp,240
    80004e94:	8082                	ret

0000000080004e96 <sys_open>:
    80004e96:	7131                	add	sp,sp,-192
    80004e98:	fd06                	sd	ra,184(sp)
    80004e9a:	f922                	sd	s0,176(sp)
    80004e9c:	f526                	sd	s1,168(sp)
    80004e9e:	f14a                	sd	s2,160(sp)
    80004ea0:	ed4e                	sd	s3,152(sp)
    80004ea2:	0180                	add	s0,sp,192
    80004ea4:	f4c40593          	add	a1,s0,-180
    80004ea8:	4505                	li	a0,1
    80004eaa:	8edfd0ef          	jal	80002796 <argint>
    80004eae:	08000613          	li	a2,128
    80004eb2:	f5040593          	add	a1,s0,-176
    80004eb6:	4501                	li	a0,0
    80004eb8:	917fd0ef          	jal	800027ce <argstr>
    80004ebc:	597d                	li	s2,-1
    80004ebe:	08054c63          	bltz	a0,80004f56 <sys_open+0xc0>
    80004ec2:	ce3fe0ef          	jal	80003ba4 <begin_op>
    80004ec6:	f4c42783          	lw	a5,-180(s0)
    80004eca:	2007f793          	and	a5,a5,512
    80004ece:	cfd9                	beqz	a5,80004f6c <sys_open+0xd6>
    80004ed0:	4681                	li	a3,0
    80004ed2:	4601                	li	a2,0
    80004ed4:	4589                	li	a1,2
    80004ed6:	f5040513          	add	a0,s0,-176
    80004eda:	ac7ff0ef          	jal	800049a0 <create>
    80004ede:	84aa                	mv	s1,a0
    80004ee0:	c159                	beqz	a0,80004f66 <sys_open+0xd0>
    80004ee2:	04449703          	lh	a4,68(s1)
    80004ee6:	478d                	li	a5,3
    80004ee8:	00f71763          	bne	a4,a5,80004ef6 <sys_open+0x60>
    80004eec:	0464d703          	lhu	a4,70(s1)
    80004ef0:	47a5                	li	a5,9
    80004ef2:	0ae7e863          	bltu	a5,a4,80004fa2 <sys_open+0x10c>
    80004ef6:	838ff0ef          	jal	80003f2e <filealloc>
    80004efa:	89aa                	mv	s3,a0
    80004efc:	0c050863          	beqz	a0,80004fcc <sys_open+0x136>
    80004f00:	a5dff0ef          	jal	8000495c <fdalloc>
    80004f04:	892a                	mv	s2,a0
    80004f06:	0c054063          	bltz	a0,80004fc6 <sys_open+0x130>
    80004f0a:	04449703          	lh	a4,68(s1)
    80004f0e:	478d                	li	a5,3
    80004f10:	0af70063          	beq	a4,a5,80004fb0 <sys_open+0x11a>
    80004f14:	4789                	li	a5,2
    80004f16:	00f9a023          	sw	a5,0(s3)
    80004f1a:	0209a023          	sw	zero,32(s3)
    80004f1e:	0099bc23          	sd	s1,24(s3)
    80004f22:	f4c42783          	lw	a5,-180(s0)
    80004f26:	0017c713          	xor	a4,a5,1
    80004f2a:	8b05                	and	a4,a4,1
    80004f2c:	00e98423          	sb	a4,8(s3)
    80004f30:	0037f713          	and	a4,a5,3
    80004f34:	00e03733          	snez	a4,a4
    80004f38:	00e984a3          	sb	a4,9(s3)
    80004f3c:	4007f793          	and	a5,a5,1024
    80004f40:	c791                	beqz	a5,80004f4c <sys_open+0xb6>
    80004f42:	04449703          	lh	a4,68(s1)
    80004f46:	4789                	li	a5,2
    80004f48:	06f70b63          	beq	a4,a5,80004fbe <sys_open+0x128>
    80004f4c:	8526                	mv	a0,s1
    80004f4e:	b18fe0ef          	jal	80003266 <iunlock>
    80004f52:	cc3fe0ef          	jal	80003c14 <end_op>
    80004f56:	854a                	mv	a0,s2
    80004f58:	70ea                	ld	ra,184(sp)
    80004f5a:	744a                	ld	s0,176(sp)
    80004f5c:	74aa                	ld	s1,168(sp)
    80004f5e:	790a                	ld	s2,160(sp)
    80004f60:	69ea                	ld	s3,152(sp)
    80004f62:	6129                	add	sp,sp,192
    80004f64:	8082                	ret
    80004f66:	caffe0ef          	jal	80003c14 <end_op>
    80004f6a:	b7f5                	j	80004f56 <sys_open+0xc0>
    80004f6c:	f5040513          	add	a0,s0,-176
    80004f70:	a45fe0ef          	jal	800039b4 <namei>
    80004f74:	84aa                	mv	s1,a0
    80004f76:	c115                	beqz	a0,80004f9a <sys_open+0x104>
    80004f78:	a42fe0ef          	jal	800031ba <ilock>
    80004f7c:	04449703          	lh	a4,68(s1)
    80004f80:	4785                	li	a5,1
    80004f82:	f6f710e3          	bne	a4,a5,80004ee2 <sys_open+0x4c>
    80004f86:	f4c42783          	lw	a5,-180(s0)
    80004f8a:	d7b5                	beqz	a5,80004ef6 <sys_open+0x60>
    80004f8c:	8526                	mv	a0,s1
    80004f8e:	c34fe0ef          	jal	800033c2 <iunlockput>
    80004f92:	c83fe0ef          	jal	80003c14 <end_op>
    80004f96:	597d                	li	s2,-1
    80004f98:	bf7d                	j	80004f56 <sys_open+0xc0>
    80004f9a:	c7bfe0ef          	jal	80003c14 <end_op>
    80004f9e:	597d                	li	s2,-1
    80004fa0:	bf5d                	j	80004f56 <sys_open+0xc0>
    80004fa2:	8526                	mv	a0,s1
    80004fa4:	c1efe0ef          	jal	800033c2 <iunlockput>
    80004fa8:	c6dfe0ef          	jal	80003c14 <end_op>
    80004fac:	597d                	li	s2,-1
    80004fae:	b765                	j	80004f56 <sys_open+0xc0>
    80004fb0:	00f9a023          	sw	a5,0(s3)
    80004fb4:	04649783          	lh	a5,70(s1)
    80004fb8:	02f99223          	sh	a5,36(s3)
    80004fbc:	b78d                	j	80004f1e <sys_open+0x88>
    80004fbe:	8526                	mv	a0,s1
    80004fc0:	ae6fe0ef          	jal	800032a6 <itrunc>
    80004fc4:	b761                	j	80004f4c <sys_open+0xb6>
    80004fc6:	854e                	mv	a0,s3
    80004fc8:	81eff0ef          	jal	80003fe6 <fileclose>
    80004fcc:	8526                	mv	a0,s1
    80004fce:	bf4fe0ef          	jal	800033c2 <iunlockput>
    80004fd2:	c43fe0ef          	jal	80003c14 <end_op>
    80004fd6:	597d                	li	s2,-1
    80004fd8:	bfbd                	j	80004f56 <sys_open+0xc0>

0000000080004fda <sys_mkdir>:
    80004fda:	7175                	add	sp,sp,-144
    80004fdc:	e506                	sd	ra,136(sp)
    80004fde:	e122                	sd	s0,128(sp)
    80004fe0:	0900                	add	s0,sp,144
    80004fe2:	bc3fe0ef          	jal	80003ba4 <begin_op>
    80004fe6:	08000613          	li	a2,128
    80004fea:	f7040593          	add	a1,s0,-144
    80004fee:	4501                	li	a0,0
    80004ff0:	fdefd0ef          	jal	800027ce <argstr>
    80004ff4:	02054363          	bltz	a0,8000501a <sys_mkdir+0x40>
    80004ff8:	4681                	li	a3,0
    80004ffa:	4601                	li	a2,0
    80004ffc:	4585                	li	a1,1
    80004ffe:	f7040513          	add	a0,s0,-144
    80005002:	99fff0ef          	jal	800049a0 <create>
    80005006:	c911                	beqz	a0,8000501a <sys_mkdir+0x40>
    80005008:	bbafe0ef          	jal	800033c2 <iunlockput>
    8000500c:	c09fe0ef          	jal	80003c14 <end_op>
    80005010:	4501                	li	a0,0
    80005012:	60aa                	ld	ra,136(sp)
    80005014:	640a                	ld	s0,128(sp)
    80005016:	6149                	add	sp,sp,144
    80005018:	8082                	ret
    8000501a:	bfbfe0ef          	jal	80003c14 <end_op>
    8000501e:	557d                	li	a0,-1
    80005020:	bfcd                	j	80005012 <sys_mkdir+0x38>

0000000080005022 <sys_mknod>:
    80005022:	7135                	add	sp,sp,-160
    80005024:	ed06                	sd	ra,152(sp)
    80005026:	e922                	sd	s0,144(sp)
    80005028:	1100                	add	s0,sp,160
    8000502a:	b7bfe0ef          	jal	80003ba4 <begin_op>
    8000502e:	f6c40593          	add	a1,s0,-148
    80005032:	4505                	li	a0,1
    80005034:	f62fd0ef          	jal	80002796 <argint>
    80005038:	f6840593          	add	a1,s0,-152
    8000503c:	4509                	li	a0,2
    8000503e:	f58fd0ef          	jal	80002796 <argint>
    80005042:	08000613          	li	a2,128
    80005046:	f7040593          	add	a1,s0,-144
    8000504a:	4501                	li	a0,0
    8000504c:	f82fd0ef          	jal	800027ce <argstr>
    80005050:	02054563          	bltz	a0,8000507a <sys_mknod+0x58>
    80005054:	f6841683          	lh	a3,-152(s0)
    80005058:	f6c41603          	lh	a2,-148(s0)
    8000505c:	458d                	li	a1,3
    8000505e:	f7040513          	add	a0,s0,-144
    80005062:	93fff0ef          	jal	800049a0 <create>
    80005066:	c911                	beqz	a0,8000507a <sys_mknod+0x58>
    80005068:	b5afe0ef          	jal	800033c2 <iunlockput>
    8000506c:	ba9fe0ef          	jal	80003c14 <end_op>
    80005070:	4501                	li	a0,0
    80005072:	60ea                	ld	ra,152(sp)
    80005074:	644a                	ld	s0,144(sp)
    80005076:	610d                	add	sp,sp,160
    80005078:	8082                	ret
    8000507a:	b9bfe0ef          	jal	80003c14 <end_op>
    8000507e:	557d                	li	a0,-1
    80005080:	bfcd                	j	80005072 <sys_mknod+0x50>

0000000080005082 <sys_chdir>:
    80005082:	7135                	add	sp,sp,-160
    80005084:	ed06                	sd	ra,152(sp)
    80005086:	e922                	sd	s0,144(sp)
    80005088:	e526                	sd	s1,136(sp)
    8000508a:	e14a                	sd	s2,128(sp)
    8000508c:	1100                	add	s0,sp,160
    8000508e:	ffefc0ef          	jal	8000188c <myproc>
    80005092:	892a                	mv	s2,a0
    80005094:	b11fe0ef          	jal	80003ba4 <begin_op>
    80005098:	08000613          	li	a2,128
    8000509c:	f6040593          	add	a1,s0,-160
    800050a0:	4501                	li	a0,0
    800050a2:	f2cfd0ef          	jal	800027ce <argstr>
    800050a6:	04054163          	bltz	a0,800050e8 <sys_chdir+0x66>
    800050aa:	f6040513          	add	a0,s0,-160
    800050ae:	907fe0ef          	jal	800039b4 <namei>
    800050b2:	84aa                	mv	s1,a0
    800050b4:	c915                	beqz	a0,800050e8 <sys_chdir+0x66>
    800050b6:	904fe0ef          	jal	800031ba <ilock>
    800050ba:	04449703          	lh	a4,68(s1)
    800050be:	4785                	li	a5,1
    800050c0:	02f71863          	bne	a4,a5,800050f0 <sys_chdir+0x6e>
    800050c4:	8526                	mv	a0,s1
    800050c6:	9a0fe0ef          	jal	80003266 <iunlock>
    800050ca:	15093503          	ld	a0,336(s2)
    800050ce:	a6cfe0ef          	jal	8000333a <iput>
    800050d2:	b43fe0ef          	jal	80003c14 <end_op>
    800050d6:	14993823          	sd	s1,336(s2)
    800050da:	4501                	li	a0,0
    800050dc:	60ea                	ld	ra,152(sp)
    800050de:	644a                	ld	s0,144(sp)
    800050e0:	64aa                	ld	s1,136(sp)
    800050e2:	690a                	ld	s2,128(sp)
    800050e4:	610d                	add	sp,sp,160
    800050e6:	8082                	ret
    800050e8:	b2dfe0ef          	jal	80003c14 <end_op>
    800050ec:	557d                	li	a0,-1
    800050ee:	b7fd                	j	800050dc <sys_chdir+0x5a>
    800050f0:	8526                	mv	a0,s1
    800050f2:	ad0fe0ef          	jal	800033c2 <iunlockput>
    800050f6:	b1ffe0ef          	jal	80003c14 <end_op>
    800050fa:	557d                	li	a0,-1
    800050fc:	b7c5                	j	800050dc <sys_chdir+0x5a>

00000000800050fe <sys_exec>:
    800050fe:	7145                	add	sp,sp,-464
    80005100:	e786                	sd	ra,456(sp)
    80005102:	e3a2                	sd	s0,448(sp)
    80005104:	ff26                	sd	s1,440(sp)
    80005106:	fb4a                	sd	s2,432(sp)
    80005108:	f74e                	sd	s3,424(sp)
    8000510a:	f352                	sd	s4,416(sp)
    8000510c:	ef56                	sd	s5,408(sp)
    8000510e:	0b80                	add	s0,sp,464
    80005110:	e3840593          	add	a1,s0,-456
    80005114:	4505                	li	a0,1
    80005116:	e9cfd0ef          	jal	800027b2 <argaddr>
    8000511a:	08000613          	li	a2,128
    8000511e:	f4040593          	add	a1,s0,-192
    80005122:	4501                	li	a0,0
    80005124:	eaafd0ef          	jal	800027ce <argstr>
    80005128:	597d                	li	s2,-1
    8000512a:	0c054163          	bltz	a0,800051ec <sys_exec+0xee>
    8000512e:	e4040913          	add	s2,s0,-448
    80005132:	10000613          	li	a2,256
    80005136:	4581                	li	a1,0
    80005138:	854a                	mv	a0,s2
    8000513a:	b51fb0ef          	jal	80000c8a <memset>
    8000513e:	89ca                	mv	s3,s2
    80005140:	4481                	li	s1,0
    80005142:	02000a93          	li	s5,32
    80005146:	00048a1b          	sext.w	s4,s1
    8000514a:	00349513          	sll	a0,s1,0x3
    8000514e:	e3040593          	add	a1,s0,-464
    80005152:	e3843783          	ld	a5,-456(s0)
    80005156:	953e                	add	a0,a0,a5
    80005158:	db4fd0ef          	jal	8000270c <fetchaddr>
    8000515c:	02054663          	bltz	a0,80005188 <sys_exec+0x8a>
    80005160:	e3043783          	ld	a5,-464(s0)
    80005164:	c7a9                	beqz	a5,800051ae <sys_exec+0xb0>
    80005166:	981fb0ef          	jal	80000ae6 <kalloc>
    8000516a:	00a93023          	sd	a0,0(s2)
    8000516e:	cd09                	beqz	a0,80005188 <sys_exec+0x8a>
    80005170:	6605                	lui	a2,0x1
    80005172:	85aa                	mv	a1,a0
    80005174:	e3043503          	ld	a0,-464(s0)
    80005178:	ddefd0ef          	jal	80002756 <fetchstr>
    8000517c:	00054663          	bltz	a0,80005188 <sys_exec+0x8a>
    80005180:	0485                	add	s1,s1,1
    80005182:	0921                	add	s2,s2,8
    80005184:	fd5491e3          	bne	s1,s5,80005146 <sys_exec+0x48>
    80005188:	e4043503          	ld	a0,-448(s0)
    8000518c:	597d                	li	s2,-1
    8000518e:	cd39                	beqz	a0,800051ec <sys_exec+0xee>
    80005190:	875fb0ef          	jal	80000a04 <kfree>
    80005194:	e4840493          	add	s1,s0,-440
    80005198:	10098993          	add	s3,s3,256
    8000519c:	6088                	ld	a0,0(s1)
    8000519e:	c531                	beqz	a0,800051ea <sys_exec+0xec>
    800051a0:	865fb0ef          	jal	80000a04 <kfree>
    800051a4:	04a1                	add	s1,s1,8
    800051a6:	ff349be3          	bne	s1,s3,8000519c <sys_exec+0x9e>
    800051aa:	597d                	li	s2,-1
    800051ac:	a081                	j	800051ec <sys_exec+0xee>
    800051ae:	0a0e                	sll	s4,s4,0x3
    800051b0:	fc040793          	add	a5,s0,-64
    800051b4:	9a3e                	add	s4,s4,a5
    800051b6:	e80a3023          	sd	zero,-384(s4)
    800051ba:	e4040593          	add	a1,s0,-448
    800051be:	f4040513          	add	a0,s0,-192
    800051c2:	bf2ff0ef          	jal	800045b4 <kexec>
    800051c6:	892a                	mv	s2,a0
    800051c8:	e4043503          	ld	a0,-448(s0)
    800051cc:	c105                	beqz	a0,800051ec <sys_exec+0xee>
    800051ce:	837fb0ef          	jal	80000a04 <kfree>
    800051d2:	e4840493          	add	s1,s0,-440
    800051d6:	10098993          	add	s3,s3,256
    800051da:	6088                	ld	a0,0(s1)
    800051dc:	c901                	beqz	a0,800051ec <sys_exec+0xee>
    800051de:	827fb0ef          	jal	80000a04 <kfree>
    800051e2:	04a1                	add	s1,s1,8
    800051e4:	ff349be3          	bne	s1,s3,800051da <sys_exec+0xdc>
    800051e8:	a011                	j	800051ec <sys_exec+0xee>
    800051ea:	597d                	li	s2,-1
    800051ec:	854a                	mv	a0,s2
    800051ee:	60be                	ld	ra,456(sp)
    800051f0:	641e                	ld	s0,448(sp)
    800051f2:	74fa                	ld	s1,440(sp)
    800051f4:	795a                	ld	s2,432(sp)
    800051f6:	79ba                	ld	s3,424(sp)
    800051f8:	7a1a                	ld	s4,416(sp)
    800051fa:	6afa                	ld	s5,408(sp)
    800051fc:	6179                	add	sp,sp,464
    800051fe:	8082                	ret

0000000080005200 <sys_pipe>:
    80005200:	7139                	add	sp,sp,-64
    80005202:	fc06                	sd	ra,56(sp)
    80005204:	f822                	sd	s0,48(sp)
    80005206:	f426                	sd	s1,40(sp)
    80005208:	0080                	add	s0,sp,64
    8000520a:	e82fc0ef          	jal	8000188c <myproc>
    8000520e:	84aa                	mv	s1,a0
    80005210:	fd840593          	add	a1,s0,-40
    80005214:	4501                	li	a0,0
    80005216:	d9cfd0ef          	jal	800027b2 <argaddr>
    8000521a:	fc840593          	add	a1,s0,-56
    8000521e:	fd040513          	add	a0,s0,-48
    80005222:	884ff0ef          	jal	800042a6 <pipealloc>
    80005226:	57fd                	li	a5,-1
    80005228:	0a054463          	bltz	a0,800052d0 <sys_pipe+0xd0>
    8000522c:	fcf42223          	sw	a5,-60(s0)
    80005230:	fd043503          	ld	a0,-48(s0)
    80005234:	f28ff0ef          	jal	8000495c <fdalloc>
    80005238:	fca42223          	sw	a0,-60(s0)
    8000523c:	08054163          	bltz	a0,800052be <sys_pipe+0xbe>
    80005240:	fc843503          	ld	a0,-56(s0)
    80005244:	f18ff0ef          	jal	8000495c <fdalloc>
    80005248:	fca42023          	sw	a0,-64(s0)
    8000524c:	06054063          	bltz	a0,800052ac <sys_pipe+0xac>
    80005250:	4691                	li	a3,4
    80005252:	fc440613          	add	a2,s0,-60
    80005256:	fd843583          	ld	a1,-40(s0)
    8000525a:	68a8                	ld	a0,80(s1)
    8000525c:	b7afc0ef          	jal	800015d6 <copyout>
    80005260:	00054e63          	bltz	a0,8000527c <sys_pipe+0x7c>
    80005264:	4691                	li	a3,4
    80005266:	fc040613          	add	a2,s0,-64
    8000526a:	fd843583          	ld	a1,-40(s0)
    8000526e:	0591                	add	a1,a1,4
    80005270:	68a8                	ld	a0,80(s1)
    80005272:	b64fc0ef          	jal	800015d6 <copyout>
    80005276:	4781                	li	a5,0
    80005278:	04055c63          	bgez	a0,800052d0 <sys_pipe+0xd0>
    8000527c:	fc442783          	lw	a5,-60(s0)
    80005280:	07e9                	add	a5,a5,26
    80005282:	078e                	sll	a5,a5,0x3
    80005284:	97a6                	add	a5,a5,s1
    80005286:	0007b023          	sd	zero,0(a5)
    8000528a:	fc042783          	lw	a5,-64(s0)
    8000528e:	07e9                	add	a5,a5,26
    80005290:	078e                	sll	a5,a5,0x3
    80005292:	94be                	add	s1,s1,a5
    80005294:	0004b023          	sd	zero,0(s1)
    80005298:	fd043503          	ld	a0,-48(s0)
    8000529c:	d4bfe0ef          	jal	80003fe6 <fileclose>
    800052a0:	fc843503          	ld	a0,-56(s0)
    800052a4:	d43fe0ef          	jal	80003fe6 <fileclose>
    800052a8:	57fd                	li	a5,-1
    800052aa:	a01d                	j	800052d0 <sys_pipe+0xd0>
    800052ac:	fc442783          	lw	a5,-60(s0)
    800052b0:	0007c763          	bltz	a5,800052be <sys_pipe+0xbe>
    800052b4:	07e9                	add	a5,a5,26
    800052b6:	078e                	sll	a5,a5,0x3
    800052b8:	94be                	add	s1,s1,a5
    800052ba:	0004b023          	sd	zero,0(s1)
    800052be:	fd043503          	ld	a0,-48(s0)
    800052c2:	d25fe0ef          	jal	80003fe6 <fileclose>
    800052c6:	fc843503          	ld	a0,-56(s0)
    800052ca:	d1dfe0ef          	jal	80003fe6 <fileclose>
    800052ce:	57fd                	li	a5,-1
    800052d0:	853e                	mv	a0,a5
    800052d2:	70e2                	ld	ra,56(sp)
    800052d4:	7442                	ld	s0,48(sp)
    800052d6:	74a2                	ld	s1,40(sp)
    800052d8:	6121                	add	sp,sp,64
    800052da:	8082                	ret
    800052dc:	0000                	unimp
	...

00000000800052e0 <kernelvec>:
    800052e0:	7111                	add	sp,sp,-256
    800052e2:	e006                	sd	ra,0(sp)
    800052e4:	e80e                	sd	gp,16(sp)
    800052e6:	ec12                	sd	tp,24(sp)
    800052e8:	f016                	sd	t0,32(sp)
    800052ea:	f41a                	sd	t1,40(sp)
    800052ec:	f81e                	sd	t2,48(sp)
    800052ee:	e4aa                	sd	a0,72(sp)
    800052f0:	e8ae                	sd	a1,80(sp)
    800052f2:	ecb2                	sd	a2,88(sp)
    800052f4:	f0b6                	sd	a3,96(sp)
    800052f6:	f4ba                	sd	a4,104(sp)
    800052f8:	f8be                	sd	a5,112(sp)
    800052fa:	fcc2                	sd	a6,120(sp)
    800052fc:	e146                	sd	a7,128(sp)
    800052fe:	edf2                	sd	t3,216(sp)
    80005300:	f1f6                	sd	t4,224(sp)
    80005302:	f5fa                	sd	t5,232(sp)
    80005304:	f9fe                	sd	t6,240(sp)
    80005306:	b16fd0ef          	jal	8000261c <kerneltrap>
    8000530a:	6082                	ld	ra,0(sp)
    8000530c:	61c2                	ld	gp,16(sp)
    8000530e:	7282                	ld	t0,32(sp)
    80005310:	7322                	ld	t1,40(sp)
    80005312:	73c2                	ld	t2,48(sp)
    80005314:	6526                	ld	a0,72(sp)
    80005316:	65c6                	ld	a1,80(sp)
    80005318:	6666                	ld	a2,88(sp)
    8000531a:	7686                	ld	a3,96(sp)
    8000531c:	7726                	ld	a4,104(sp)
    8000531e:	77c6                	ld	a5,112(sp)
    80005320:	7866                	ld	a6,120(sp)
    80005322:	688a                	ld	a7,128(sp)
    80005324:	6e6e                	ld	t3,216(sp)
    80005326:	7e8e                	ld	t4,224(sp)
    80005328:	7f2e                	ld	t5,232(sp)
    8000532a:	7fce                	ld	t6,240(sp)
    8000532c:	6111                	add	sp,sp,256
    8000532e:	10200073          	sret
	...

000000008000533e <plicinit>:
    8000533e:	1141                	add	sp,sp,-16
    80005340:	e422                	sd	s0,8(sp)
    80005342:	0800                	add	s0,sp,16
    80005344:	0c0007b7          	lui	a5,0xc000
    80005348:	4705                	li	a4,1
    8000534a:	d798                	sw	a4,40(a5)
    8000534c:	c3d8                	sw	a4,4(a5)
    8000534e:	6422                	ld	s0,8(sp)
    80005350:	0141                	add	sp,sp,16
    80005352:	8082                	ret

0000000080005354 <plicinithart>:
    80005354:	1141                	add	sp,sp,-16
    80005356:	e406                	sd	ra,8(sp)
    80005358:	e022                	sd	s0,0(sp)
    8000535a:	0800                	add	s0,sp,16
    8000535c:	d04fc0ef          	jal	80001860 <cpuid>
    80005360:	0085171b          	sllw	a4,a0,0x8
    80005364:	0c0027b7          	lui	a5,0xc002
    80005368:	97ba                	add	a5,a5,a4
    8000536a:	40200713          	li	a4,1026
    8000536e:	08e7a023          	sw	a4,128(a5) # c002080 <_entry-0x73ffdf80>
    80005372:	00d5151b          	sllw	a0,a0,0xd
    80005376:	0c2017b7          	lui	a5,0xc201
    8000537a:	953e                	add	a0,a0,a5
    8000537c:	00052023          	sw	zero,0(a0)
    80005380:	60a2                	ld	ra,8(sp)
    80005382:	6402                	ld	s0,0(sp)
    80005384:	0141                	add	sp,sp,16
    80005386:	8082                	ret

0000000080005388 <plic_claim>:
    80005388:	1141                	add	sp,sp,-16
    8000538a:	e406                	sd	ra,8(sp)
    8000538c:	e022                	sd	s0,0(sp)
    8000538e:	0800                	add	s0,sp,16
    80005390:	cd0fc0ef          	jal	80001860 <cpuid>
    80005394:	00d5151b          	sllw	a0,a0,0xd
    80005398:	0c2017b7          	lui	a5,0xc201
    8000539c:	97aa                	add	a5,a5,a0
    8000539e:	43c8                	lw	a0,4(a5)
    800053a0:	60a2                	ld	ra,8(sp)
    800053a2:	6402                	ld	s0,0(sp)
    800053a4:	0141                	add	sp,sp,16
    800053a6:	8082                	ret

00000000800053a8 <plic_complete>:
    800053a8:	1101                	add	sp,sp,-32
    800053aa:	ec06                	sd	ra,24(sp)
    800053ac:	e822                	sd	s0,16(sp)
    800053ae:	e426                	sd	s1,8(sp)
    800053b0:	1000                	add	s0,sp,32
    800053b2:	84aa                	mv	s1,a0
    800053b4:	cacfc0ef          	jal	80001860 <cpuid>
    800053b8:	00d5151b          	sllw	a0,a0,0xd
    800053bc:	0c2017b7          	lui	a5,0xc201
    800053c0:	97aa                	add	a5,a5,a0
    800053c2:	c3c4                	sw	s1,4(a5)
    800053c4:	60e2                	ld	ra,24(sp)
    800053c6:	6442                	ld	s0,16(sp)
    800053c8:	64a2                	ld	s1,8(sp)
    800053ca:	6105                	add	sp,sp,32
    800053cc:	8082                	ret

00000000800053ce <free_desc>:
    800053ce:	1141                	add	sp,sp,-16
    800053d0:	e406                	sd	ra,8(sp)
    800053d2:	e022                	sd	s0,0(sp)
    800053d4:	0800                	add	s0,sp,16
    800053d6:	479d                	li	a5,7
    800053d8:	04a7ca63          	blt	a5,a0,8000542c <free_desc+0x5e>
    800053dc:	0001b797          	auipc	a5,0x1b
    800053e0:	64c78793          	add	a5,a5,1612 # 80020a28 <disk>
    800053e4:	97aa                	add	a5,a5,a0
    800053e6:	0187c783          	lbu	a5,24(a5)
    800053ea:	e7b9                	bnez	a5,80005438 <free_desc+0x6a>
    800053ec:	00451613          	sll	a2,a0,0x4
    800053f0:	0001b797          	auipc	a5,0x1b
    800053f4:	63878793          	add	a5,a5,1592 # 80020a28 <disk>
    800053f8:	6394                	ld	a3,0(a5)
    800053fa:	96b2                	add	a3,a3,a2
    800053fc:	0006b023          	sd	zero,0(a3)
    80005400:	6398                	ld	a4,0(a5)
    80005402:	9732                	add	a4,a4,a2
    80005404:	00072423          	sw	zero,8(a4)
    80005408:	00071623          	sh	zero,12(a4)
    8000540c:	00071723          	sh	zero,14(a4)
    80005410:	97aa                	add	a5,a5,a0
    80005412:	4705                	li	a4,1
    80005414:	00e78c23          	sb	a4,24(a5)
    80005418:	0001b517          	auipc	a0,0x1b
    8000541c:	62850513          	add	a0,a0,1576 # 80020a40 <disk+0x18>
    80005420:	ac1fc0ef          	jal	80001ee0 <wakeup>
    80005424:	60a2                	ld	ra,8(sp)
    80005426:	6402                	ld	s0,0(sp)
    80005428:	0141                	add	sp,sp,16
    8000542a:	8082                	ret
    8000542c:	00002517          	auipc	a0,0x2
    80005430:	2f450513          	add	a0,a0,756 # 80007720 <syscalls+0x358>
    80005434:	b8efb0ef          	jal	800007c2 <panic>
    80005438:	00002517          	auipc	a0,0x2
    8000543c:	2f850513          	add	a0,a0,760 # 80007730 <syscalls+0x368>
    80005440:	b82fb0ef          	jal	800007c2 <panic>

0000000080005444 <virtio_disk_init>:
    80005444:	1101                	add	sp,sp,-32
    80005446:	ec06                	sd	ra,24(sp)
    80005448:	e822                	sd	s0,16(sp)
    8000544a:	e426                	sd	s1,8(sp)
    8000544c:	e04a                	sd	s2,0(sp)
    8000544e:	1000                	add	s0,sp,32
    80005450:	00002597          	auipc	a1,0x2
    80005454:	2f058593          	add	a1,a1,752 # 80007740 <syscalls+0x378>
    80005458:	0001b517          	auipc	a0,0x1b
    8000545c:	6f850513          	add	a0,a0,1784 # 80020b50 <disk+0x128>
    80005460:	ed6fb0ef          	jal	80000b36 <initlock>
    80005464:	100017b7          	lui	a5,0x10001
    80005468:	4398                	lw	a4,0(a5)
    8000546a:	2701                	sext.w	a4,a4
    8000546c:	747277b7          	lui	a5,0x74727
    80005470:	97678793          	add	a5,a5,-1674 # 74726976 <_entry-0xb8d968a>
    80005474:	12f71e63          	bne	a4,a5,800055b0 <virtio_disk_init+0x16c>
    80005478:	100017b7          	lui	a5,0x10001
    8000547c:	43dc                	lw	a5,4(a5)
    8000547e:	2781                	sext.w	a5,a5
    80005480:	4709                	li	a4,2
    80005482:	12e79763          	bne	a5,a4,800055b0 <virtio_disk_init+0x16c>
    80005486:	100017b7          	lui	a5,0x10001
    8000548a:	479c                	lw	a5,8(a5)
    8000548c:	2781                	sext.w	a5,a5
    8000548e:	12e79163          	bne	a5,a4,800055b0 <virtio_disk_init+0x16c>
    80005492:	100017b7          	lui	a5,0x10001
    80005496:	47d8                	lw	a4,12(a5)
    80005498:	2701                	sext.w	a4,a4
    8000549a:	554d47b7          	lui	a5,0x554d4
    8000549e:	55178793          	add	a5,a5,1361 # 554d4551 <_entry-0x2ab2baaf>
    800054a2:	10f71763          	bne	a4,a5,800055b0 <virtio_disk_init+0x16c>
    800054a6:	100017b7          	lui	a5,0x10001
    800054aa:	0607a823          	sw	zero,112(a5) # 10001070 <_entry-0x6fffef90>
    800054ae:	4705                	li	a4,1
    800054b0:	dbb8                	sw	a4,112(a5)
    800054b2:	470d                	li	a4,3
    800054b4:	dbb8                	sw	a4,112(a5)
    800054b6:	4b94                	lw	a3,16(a5)
    800054b8:	c7ffe737          	lui	a4,0xc7ffe
    800054bc:	75f70713          	add	a4,a4,1887 # ffffffffc7ffe75f <end+0xffffffff47fddbf7>
    800054c0:	8f75                	and	a4,a4,a3
    800054c2:	2701                	sext.w	a4,a4
    800054c4:	d398                	sw	a4,32(a5)
    800054c6:	472d                	li	a4,11
    800054c8:	dbb8                	sw	a4,112(a5)
    800054ca:	0707a903          	lw	s2,112(a5)
    800054ce:	2901                	sext.w	s2,s2
    800054d0:	00897793          	and	a5,s2,8
    800054d4:	0e078463          	beqz	a5,800055bc <virtio_disk_init+0x178>
    800054d8:	100017b7          	lui	a5,0x10001
    800054dc:	0207a823          	sw	zero,48(a5) # 10001030 <_entry-0x6fffefd0>
    800054e0:	43fc                	lw	a5,68(a5)
    800054e2:	2781                	sext.w	a5,a5
    800054e4:	0e079263          	bnez	a5,800055c8 <virtio_disk_init+0x184>
    800054e8:	100017b7          	lui	a5,0x10001
    800054ec:	5bdc                	lw	a5,52(a5)
    800054ee:	2781                	sext.w	a5,a5
    800054f0:	0e078263          	beqz	a5,800055d4 <virtio_disk_init+0x190>
    800054f4:	471d                	li	a4,7
    800054f6:	0ef77563          	bgeu	a4,a5,800055e0 <virtio_disk_init+0x19c>
    800054fa:	decfb0ef          	jal	80000ae6 <kalloc>
    800054fe:	0001b497          	auipc	s1,0x1b
    80005502:	52a48493          	add	s1,s1,1322 # 80020a28 <disk>
    80005506:	e088                	sd	a0,0(s1)
    80005508:	ddefb0ef          	jal	80000ae6 <kalloc>
    8000550c:	e488                	sd	a0,8(s1)
    8000550e:	dd8fb0ef          	jal	80000ae6 <kalloc>
    80005512:	e888                	sd	a0,16(s1)
    80005514:	609c                	ld	a5,0(s1)
    80005516:	cbf9                	beqz	a5,800055ec <virtio_disk_init+0x1a8>
    80005518:	6498                	ld	a4,8(s1)
    8000551a:	cb69                	beqz	a4,800055ec <virtio_disk_init+0x1a8>
    8000551c:	c961                	beqz	a0,800055ec <virtio_disk_init+0x1a8>
    8000551e:	6605                	lui	a2,0x1
    80005520:	4581                	li	a1,0
    80005522:	853e                	mv	a0,a5
    80005524:	f66fb0ef          	jal	80000c8a <memset>
    80005528:	0001b497          	auipc	s1,0x1b
    8000552c:	50048493          	add	s1,s1,1280 # 80020a28 <disk>
    80005530:	6605                	lui	a2,0x1
    80005532:	4581                	li	a1,0
    80005534:	6488                	ld	a0,8(s1)
    80005536:	f54fb0ef          	jal	80000c8a <memset>
    8000553a:	6605                	lui	a2,0x1
    8000553c:	4581                	li	a1,0
    8000553e:	6888                	ld	a0,16(s1)
    80005540:	f4afb0ef          	jal	80000c8a <memset>
    80005544:	100017b7          	lui	a5,0x10001
    80005548:	4721                	li	a4,8
    8000554a:	df98                	sw	a4,56(a5)
    8000554c:	4098                	lw	a4,0(s1)
    8000554e:	08e7a023          	sw	a4,128(a5) # 10001080 <_entry-0x6fffef80>
    80005552:	40d8                	lw	a4,4(s1)
    80005554:	08e7a223          	sw	a4,132(a5)
    80005558:	6498                	ld	a4,8(s1)
    8000555a:	0007069b          	sext.w	a3,a4
    8000555e:	08d7a823          	sw	a3,144(a5)
    80005562:	9701                	sra	a4,a4,0x20
    80005564:	08e7aa23          	sw	a4,148(a5)
    80005568:	6898                	ld	a4,16(s1)
    8000556a:	0007069b          	sext.w	a3,a4
    8000556e:	0ad7a023          	sw	a3,160(a5)
    80005572:	9701                	sra	a4,a4,0x20
    80005574:	0ae7a223          	sw	a4,164(a5)
    80005578:	4705                	li	a4,1
    8000557a:	c3f8                	sw	a4,68(a5)
    8000557c:	00e48c23          	sb	a4,24(s1)
    80005580:	00e48ca3          	sb	a4,25(s1)
    80005584:	00e48d23          	sb	a4,26(s1)
    80005588:	00e48da3          	sb	a4,27(s1)
    8000558c:	00e48e23          	sb	a4,28(s1)
    80005590:	00e48ea3          	sb	a4,29(s1)
    80005594:	00e48f23          	sb	a4,30(s1)
    80005598:	00e48fa3          	sb	a4,31(s1)
    8000559c:	00496913          	or	s2,s2,4
    800055a0:	0727a823          	sw	s2,112(a5)
    800055a4:	60e2                	ld	ra,24(sp)
    800055a6:	6442                	ld	s0,16(sp)
    800055a8:	64a2                	ld	s1,8(sp)
    800055aa:	6902                	ld	s2,0(sp)
    800055ac:	6105                	add	sp,sp,32
    800055ae:	8082                	ret
    800055b0:	00002517          	auipc	a0,0x2
    800055b4:	1a050513          	add	a0,a0,416 # 80007750 <syscalls+0x388>
    800055b8:	a0afb0ef          	jal	800007c2 <panic>
    800055bc:	00002517          	auipc	a0,0x2
    800055c0:	1b450513          	add	a0,a0,436 # 80007770 <syscalls+0x3a8>
    800055c4:	9fefb0ef          	jal	800007c2 <panic>
    800055c8:	00002517          	auipc	a0,0x2
    800055cc:	1c850513          	add	a0,a0,456 # 80007790 <syscalls+0x3c8>
    800055d0:	9f2fb0ef          	jal	800007c2 <panic>
    800055d4:	00002517          	auipc	a0,0x2
    800055d8:	1dc50513          	add	a0,a0,476 # 800077b0 <syscalls+0x3e8>
    800055dc:	9e6fb0ef          	jal	800007c2 <panic>
    800055e0:	00002517          	auipc	a0,0x2
    800055e4:	1f050513          	add	a0,a0,496 # 800077d0 <syscalls+0x408>
    800055e8:	9dafb0ef          	jal	800007c2 <panic>
    800055ec:	00002517          	auipc	a0,0x2
    800055f0:	20450513          	add	a0,a0,516 # 800077f0 <syscalls+0x428>
    800055f4:	9cefb0ef          	jal	800007c2 <panic>

00000000800055f8 <virtio_disk_rw>:
    800055f8:	7159                	add	sp,sp,-112
    800055fa:	f486                	sd	ra,104(sp)
    800055fc:	f0a2                	sd	s0,96(sp)
    800055fe:	eca6                	sd	s1,88(sp)
    80005600:	e8ca                	sd	s2,80(sp)
    80005602:	e4ce                	sd	s3,72(sp)
    80005604:	e0d2                	sd	s4,64(sp)
    80005606:	fc56                	sd	s5,56(sp)
    80005608:	f85a                	sd	s6,48(sp)
    8000560a:	f45e                	sd	s7,40(sp)
    8000560c:	f062                	sd	s8,32(sp)
    8000560e:	ec66                	sd	s9,24(sp)
    80005610:	e86a                	sd	s10,16(sp)
    80005612:	1880                	add	s0,sp,112
    80005614:	892a                	mv	s2,a0
    80005616:	8c2e                	mv	s8,a1
    80005618:	00c52c83          	lw	s9,12(a0)
    8000561c:	001c9c9b          	sllw	s9,s9,0x1
    80005620:	1c82                	sll	s9,s9,0x20
    80005622:	020cdc93          	srl	s9,s9,0x20
    80005626:	0001b517          	auipc	a0,0x1b
    8000562a:	52a50513          	add	a0,a0,1322 # 80020b50 <disk+0x128>
    8000562e:	d88fb0ef          	jal	80000bb6 <acquire>
    80005632:	0001b997          	auipc	s3,0x1b
    80005636:	3f698993          	add	s3,s3,1014 # 80020a28 <disk>
    8000563a:	4d05                	li	s10,1
    8000563c:	4b21                	li	s6,8
    8000563e:	4a8d                	li	s5,3
    80005640:	8a6a                	mv	s4,s10
    80005642:	0001bb97          	auipc	s7,0x1b
    80005646:	50eb8b93          	add	s7,s7,1294 # 80020b50 <disk+0x128>
    8000564a:	a88d                	j	800056bc <virtio_disk_rw+0xc4>
    8000564c:	00f986b3          	add	a3,s3,a5
    80005650:	00068c23          	sb	zero,24(a3)
    80005654:	c21c                	sw	a5,0(a2)
    80005656:	0207c963          	bltz	a5,80005688 <virtio_disk_rw+0x90>
    8000565a:	2485                	addw	s1,s1,1
    8000565c:	0711                	add	a4,a4,4
    8000565e:	07548363          	beq	s1,s5,800056c4 <virtio_disk_rw+0xcc>
    80005662:	863a                	mv	a2,a4
    80005664:	0189c783          	lbu	a5,24(s3)
    80005668:	1c079063          	bnez	a5,80005828 <virtio_disk_rw+0x230>
    8000566c:	0001b697          	auipc	a3,0x1b
    80005670:	3bc68693          	add	a3,a3,956 # 80020a28 <disk>
    80005674:	87d2                	mv	a5,s4
    80005676:	0196c583          	lbu	a1,25(a3)
    8000567a:	f9e9                	bnez	a1,8000564c <virtio_disk_rw+0x54>
    8000567c:	2785                	addw	a5,a5,1
    8000567e:	0685                	add	a3,a3,1
    80005680:	ff679be3          	bne	a5,s6,80005676 <virtio_disk_rw+0x7e>
    80005684:	57fd                	li	a5,-1
    80005686:	c21c                	sw	a5,0(a2)
    80005688:	02905363          	blez	s1,800056ae <virtio_disk_rw+0xb6>
    8000568c:	f9042503          	lw	a0,-112(s0)
    80005690:	d3fff0ef          	jal	800053ce <free_desc>
    80005694:	009d5d63          	bge	s10,s1,800056ae <virtio_disk_rw+0xb6>
    80005698:	f9442503          	lw	a0,-108(s0)
    8000569c:	d33ff0ef          	jal	800053ce <free_desc>
    800056a0:	4789                	li	a5,2
    800056a2:	0097d663          	bge	a5,s1,800056ae <virtio_disk_rw+0xb6>
    800056a6:	f9842503          	lw	a0,-104(s0)
    800056aa:	d25ff0ef          	jal	800053ce <free_desc>
    800056ae:	85de                	mv	a1,s7
    800056b0:	0001b517          	auipc	a0,0x1b
    800056b4:	39050513          	add	a0,a0,912 # 80020a40 <disk+0x18>
    800056b8:	fdcfc0ef          	jal	80001e94 <sleep>
    800056bc:	f9040713          	add	a4,s0,-112
    800056c0:	4481                	li	s1,0
    800056c2:	b745                	j	80005662 <virtio_disk_rw+0x6a>
    800056c4:	f9042583          	lw	a1,-112(s0)
    800056c8:	00a58793          	add	a5,a1,10
    800056cc:	0792                	sll	a5,a5,0x4
    800056ce:	0001b617          	auipc	a2,0x1b
    800056d2:	35a60613          	add	a2,a2,858 # 80020a28 <disk>
    800056d6:	00f60733          	add	a4,a2,a5
    800056da:	018036b3          	snez	a3,s8
    800056de:	c714                	sw	a3,8(a4)
    800056e0:	00072623          	sw	zero,12(a4)
    800056e4:	01973823          	sd	s9,16(a4)
    800056e8:	f6078693          	add	a3,a5,-160
    800056ec:	6218                	ld	a4,0(a2)
    800056ee:	9736                	add	a4,a4,a3
    800056f0:	00878513          	add	a0,a5,8
    800056f4:	9532                	add	a0,a0,a2
    800056f6:	e308                	sd	a0,0(a4)
    800056f8:	6208                	ld	a0,0(a2)
    800056fa:	96aa                	add	a3,a3,a0
    800056fc:	4741                	li	a4,16
    800056fe:	c698                	sw	a4,8(a3)
    80005700:	4705                	li	a4,1
    80005702:	00e69623          	sh	a4,12(a3)
    80005706:	f9442703          	lw	a4,-108(s0)
    8000570a:	00e69723          	sh	a4,14(a3)
    8000570e:	0712                	sll	a4,a4,0x4
    80005710:	953a                	add	a0,a0,a4
    80005712:	05890693          	add	a3,s2,88
    80005716:	e114                	sd	a3,0(a0)
    80005718:	6208                	ld	a0,0(a2)
    8000571a:	972a                	add	a4,a4,a0
    8000571c:	40000693          	li	a3,1024
    80005720:	c714                	sw	a3,8(a4)
    80005722:	001c3c13          	seqz	s8,s8
    80005726:	0c06                	sll	s8,s8,0x1
    80005728:	001c6c13          	or	s8,s8,1
    8000572c:	01871623          	sh	s8,12(a4)
    80005730:	f9842603          	lw	a2,-104(s0)
    80005734:	00c71723          	sh	a2,14(a4)
    80005738:	0001b697          	auipc	a3,0x1b
    8000573c:	2f068693          	add	a3,a3,752 # 80020a28 <disk>
    80005740:	00258713          	add	a4,a1,2
    80005744:	0712                	sll	a4,a4,0x4
    80005746:	9736                	add	a4,a4,a3
    80005748:	587d                	li	a6,-1
    8000574a:	01070823          	sb	a6,16(a4)
    8000574e:	0612                	sll	a2,a2,0x4
    80005750:	9532                	add	a0,a0,a2
    80005752:	f9078793          	add	a5,a5,-112
    80005756:	97b6                	add	a5,a5,a3
    80005758:	e11c                	sd	a5,0(a0)
    8000575a:	629c                	ld	a5,0(a3)
    8000575c:	97b2                	add	a5,a5,a2
    8000575e:	4605                	li	a2,1
    80005760:	c790                	sw	a2,8(a5)
    80005762:	4509                	li	a0,2
    80005764:	00a79623          	sh	a0,12(a5)
    80005768:	00079723          	sh	zero,14(a5)
    8000576c:	00c92223          	sw	a2,4(s2)
    80005770:	01273423          	sd	s2,8(a4)
    80005774:	6698                	ld	a4,8(a3)
    80005776:	00275783          	lhu	a5,2(a4)
    8000577a:	8b9d                	and	a5,a5,7
    8000577c:	0786                	sll	a5,a5,0x1
    8000577e:	97ba                	add	a5,a5,a4
    80005780:	00b79223          	sh	a1,4(a5)
    80005784:	0ff0000f          	fence
    80005788:	6698                	ld	a4,8(a3)
    8000578a:	00275783          	lhu	a5,2(a4)
    8000578e:	2785                	addw	a5,a5,1
    80005790:	00f71123          	sh	a5,2(a4)
    80005794:	0ff0000f          	fence
    80005798:	100017b7          	lui	a5,0x10001
    8000579c:	0407a823          	sw	zero,80(a5) # 10001050 <_entry-0x6fffefb0>
    800057a0:	00492783          	lw	a5,4(s2)
    800057a4:	00c79f63          	bne	a5,a2,800057c2 <virtio_disk_rw+0x1ca>
    800057a8:	0001b997          	auipc	s3,0x1b
    800057ac:	3a898993          	add	s3,s3,936 # 80020b50 <disk+0x128>
    800057b0:	4485                	li	s1,1
    800057b2:	85ce                	mv	a1,s3
    800057b4:	854a                	mv	a0,s2
    800057b6:	edefc0ef          	jal	80001e94 <sleep>
    800057ba:	00492783          	lw	a5,4(s2)
    800057be:	fe978ae3          	beq	a5,s1,800057b2 <virtio_disk_rw+0x1ba>
    800057c2:	f9042503          	lw	a0,-112(s0)
    800057c6:	00250793          	add	a5,a0,2
    800057ca:	00479713          	sll	a4,a5,0x4
    800057ce:	0001b797          	auipc	a5,0x1b
    800057d2:	25a78793          	add	a5,a5,602 # 80020a28 <disk>
    800057d6:	97ba                	add	a5,a5,a4
    800057d8:	0007b423          	sd	zero,8(a5)
    800057dc:	0001b997          	auipc	s3,0x1b
    800057e0:	24c98993          	add	s3,s3,588 # 80020a28 <disk>
    800057e4:	00451713          	sll	a4,a0,0x4
    800057e8:	0009b783          	ld	a5,0(s3)
    800057ec:	97ba                	add	a5,a5,a4
    800057ee:	00c7d483          	lhu	s1,12(a5)
    800057f2:	00e7d903          	lhu	s2,14(a5)
    800057f6:	bd9ff0ef          	jal	800053ce <free_desc>
    800057fa:	854a                	mv	a0,s2
    800057fc:	8885                	and	s1,s1,1
    800057fe:	f0fd                	bnez	s1,800057e4 <virtio_disk_rw+0x1ec>
    80005800:	0001b517          	auipc	a0,0x1b
    80005804:	35050513          	add	a0,a0,848 # 80020b50 <disk+0x128>
    80005808:	c46fb0ef          	jal	80000c4e <release>
    8000580c:	70a6                	ld	ra,104(sp)
    8000580e:	7406                	ld	s0,96(sp)
    80005810:	64e6                	ld	s1,88(sp)
    80005812:	6946                	ld	s2,80(sp)
    80005814:	69a6                	ld	s3,72(sp)
    80005816:	6a06                	ld	s4,64(sp)
    80005818:	7ae2                	ld	s5,56(sp)
    8000581a:	7b42                	ld	s6,48(sp)
    8000581c:	7ba2                	ld	s7,40(sp)
    8000581e:	7c02                	ld	s8,32(sp)
    80005820:	6ce2                	ld	s9,24(sp)
    80005822:	6d42                	ld	s10,16(sp)
    80005824:	6165                	add	sp,sp,112
    80005826:	8082                	ret
    80005828:	00098c23          	sb	zero,24(s3)
    8000582c:	00072023          	sw	zero,0(a4)
    80005830:	b52d                	j	8000565a <virtio_disk_rw+0x62>

0000000080005832 <virtio_disk_intr>:
    80005832:	1101                	add	sp,sp,-32
    80005834:	ec06                	sd	ra,24(sp)
    80005836:	e822                	sd	s0,16(sp)
    80005838:	e426                	sd	s1,8(sp)
    8000583a:	1000                	add	s0,sp,32
    8000583c:	0001b497          	auipc	s1,0x1b
    80005840:	1ec48493          	add	s1,s1,492 # 80020a28 <disk>
    80005844:	0001b517          	auipc	a0,0x1b
    80005848:	30c50513          	add	a0,a0,780 # 80020b50 <disk+0x128>
    8000584c:	b6afb0ef          	jal	80000bb6 <acquire>
    80005850:	10001737          	lui	a4,0x10001
    80005854:	533c                	lw	a5,96(a4)
    80005856:	8b8d                	and	a5,a5,3
    80005858:	d37c                	sw	a5,100(a4)
    8000585a:	0ff0000f          	fence
    8000585e:	689c                	ld	a5,16(s1)
    80005860:	0204d703          	lhu	a4,32(s1)
    80005864:	0027d783          	lhu	a5,2(a5)
    80005868:	04f70663          	beq	a4,a5,800058b4 <virtio_disk_intr+0x82>
    8000586c:	0ff0000f          	fence
    80005870:	6898                	ld	a4,16(s1)
    80005872:	0204d783          	lhu	a5,32(s1)
    80005876:	8b9d                	and	a5,a5,7
    80005878:	078e                	sll	a5,a5,0x3
    8000587a:	97ba                	add	a5,a5,a4
    8000587c:	43dc                	lw	a5,4(a5)
    8000587e:	00278713          	add	a4,a5,2
    80005882:	0712                	sll	a4,a4,0x4
    80005884:	9726                	add	a4,a4,s1
    80005886:	01074703          	lbu	a4,16(a4) # 10001010 <_entry-0x6fffeff0>
    8000588a:	e321                	bnez	a4,800058ca <virtio_disk_intr+0x98>
    8000588c:	0789                	add	a5,a5,2
    8000588e:	0792                	sll	a5,a5,0x4
    80005890:	97a6                	add	a5,a5,s1
    80005892:	6788                	ld	a0,8(a5)
    80005894:	00052223          	sw	zero,4(a0)
    80005898:	e48fc0ef          	jal	80001ee0 <wakeup>
    8000589c:	0204d783          	lhu	a5,32(s1)
    800058a0:	2785                	addw	a5,a5,1
    800058a2:	17c2                	sll	a5,a5,0x30
    800058a4:	93c1                	srl	a5,a5,0x30
    800058a6:	02f49023          	sh	a5,32(s1)
    800058aa:	6898                	ld	a4,16(s1)
    800058ac:	00275703          	lhu	a4,2(a4)
    800058b0:	faf71ee3          	bne	a4,a5,8000586c <virtio_disk_intr+0x3a>
    800058b4:	0001b517          	auipc	a0,0x1b
    800058b8:	29c50513          	add	a0,a0,668 # 80020b50 <disk+0x128>
    800058bc:	b92fb0ef          	jal	80000c4e <release>
    800058c0:	60e2                	ld	ra,24(sp)
    800058c2:	6442                	ld	s0,16(sp)
    800058c4:	64a2                	ld	s1,8(sp)
    800058c6:	6105                	add	sp,sp,32
    800058c8:	8082                	ret
    800058ca:	00002517          	auipc	a0,0x2
    800058ce:	f3e50513          	add	a0,a0,-194 # 80007808 <syscalls+0x440>
    800058d2:	ef1fa0ef          	jal	800007c2 <panic>
	...

0000000080006000 <_trampoline>:
    80006000:	14051073          	csrw	sscratch,a0
    80006004:	02000537          	lui	a0,0x2000
    80006008:	fff5051b          	addw	a0,a0,-1 # 1ffffff <_entry-0x7e000001>
    8000600c:	00d51513          	sll	a0,a0,0xd
    80006010:	02153423          	sd	ra,40(a0)
    80006014:	02253823          	sd	sp,48(a0)
    80006018:	02353c23          	sd	gp,56(a0)
    8000601c:	04453023          	sd	tp,64(a0)
    80006020:	04553423          	sd	t0,72(a0)
    80006024:	04653823          	sd	t1,80(a0)
    80006028:	04753c23          	sd	t2,88(a0)
    8000602c:	f120                	sd	s0,96(a0)
    8000602e:	f524                	sd	s1,104(a0)
    80006030:	fd2c                	sd	a1,120(a0)
    80006032:	e150                	sd	a2,128(a0)
    80006034:	e554                	sd	a3,136(a0)
    80006036:	e958                	sd	a4,144(a0)
    80006038:	ed5c                	sd	a5,152(a0)
    8000603a:	0b053023          	sd	a6,160(a0)
    8000603e:	0b153423          	sd	a7,168(a0)
    80006042:	0b253823          	sd	s2,176(a0)
    80006046:	0b353c23          	sd	s3,184(a0)
    8000604a:	0d453023          	sd	s4,192(a0)
    8000604e:	0d553423          	sd	s5,200(a0)
    80006052:	0d653823          	sd	s6,208(a0)
    80006056:	0d753c23          	sd	s7,216(a0)
    8000605a:	0f853023          	sd	s8,224(a0)
    8000605e:	0f953423          	sd	s9,232(a0)
    80006062:	0fa53823          	sd	s10,240(a0)
    80006066:	0fb53c23          	sd	s11,248(a0)
    8000606a:	11c53023          	sd	t3,256(a0)
    8000606e:	11d53423          	sd	t4,264(a0)
    80006072:	11e53823          	sd	t5,272(a0)
    80006076:	11f53c23          	sd	t6,280(a0)
    8000607a:	140022f3          	csrr	t0,sscratch
    8000607e:	06553823          	sd	t0,112(a0)
    80006082:	00853103          	ld	sp,8(a0)
    80006086:	02053203          	ld	tp,32(a0)
    8000608a:	01053283          	ld	t0,16(a0)
    8000608e:	00053303          	ld	t1,0(a0)
    80006092:	12000073          	sfence.vma
    80006096:	18031073          	csrw	satp,t1
    8000609a:	12000073          	sfence.vma
    8000609e:	9282                	jalr	t0

00000000800060a0 <userret>:
    800060a0:	12000073          	sfence.vma
    800060a4:	18051073          	csrw	satp,a0
    800060a8:	12000073          	sfence.vma
    800060ac:	02000537          	lui	a0,0x2000
    800060b0:	fff5051b          	addw	a0,a0,-1 # 1ffffff <_entry-0x7e000001>
    800060b4:	00d51513          	sll	a0,a0,0xd
    800060b8:	02853083          	ld	ra,40(a0)
    800060bc:	03053103          	ld	sp,48(a0)
    800060c0:	03853183          	ld	gp,56(a0)
    800060c4:	04053203          	ld	tp,64(a0)
    800060c8:	04853283          	ld	t0,72(a0)
    800060cc:	05053303          	ld	t1,80(a0)
    800060d0:	05853383          	ld	t2,88(a0)
    800060d4:	7120                	ld	s0,96(a0)
    800060d6:	7524                	ld	s1,104(a0)
    800060d8:	7d2c                	ld	a1,120(a0)
    800060da:	6150                	ld	a2,128(a0)
    800060dc:	6554                	ld	a3,136(a0)
    800060de:	6958                	ld	a4,144(a0)
    800060e0:	6d5c                	ld	a5,152(a0)
    800060e2:	0a053803          	ld	a6,160(a0)
    800060e6:	0a853883          	ld	a7,168(a0)
    800060ea:	0b053903          	ld	s2,176(a0)
    800060ee:	0b853983          	ld	s3,184(a0)
    800060f2:	0c053a03          	ld	s4,192(a0)
    800060f6:	0c853a83          	ld	s5,200(a0)
    800060fa:	0d053b03          	ld	s6,208(a0)
    800060fe:	0d853b83          	ld	s7,216(a0)
    80006102:	0e053c03          	ld	s8,224(a0)
    80006106:	0e853c83          	ld	s9,232(a0)
    8000610a:	0f053d03          	ld	s10,240(a0)
    8000610e:	0f853d83          	ld	s11,248(a0)
    80006112:	10053e03          	ld	t3,256(a0)
    80006116:	10853e83          	ld	t4,264(a0)
    8000611a:	11053f03          	ld	t5,272(a0)
    8000611e:	11853f83          	ld	t6,280(a0)
    80006122:	7928                	ld	a0,112(a0)
    80006124:	10200073          	sret
	...
