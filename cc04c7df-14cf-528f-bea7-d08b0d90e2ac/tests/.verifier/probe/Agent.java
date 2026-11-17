package grader.probe;

import java.io.FileOutputStream;
import java.io.IOException;
import java.io.PrintStream;
import java.lang.instrument.ClassFileTransformer;
import java.lang.instrument.Instrumentation;
import java.nio.charset.StandardCharsets;
import java.security.CodeSource;
import java.security.ProtectionDomain;
import java.util.Map;
import java.util.Set;

import jdk.internal.org.objectweb.asm.ClassReader;
import jdk.internal.org.objectweb.asm.ClassVisitor;
import jdk.internal.org.objectweb.asm.ClassWriter;
import jdk.internal.org.objectweb.asm.MethodVisitor;
import jdk.internal.org.objectweb.asm.Opcodes;

/**
 * Grader-side JVM probe. Records, for every request Tomcat builds a filter chain for, the request URI and the
 * class of the servlet the chain targets; at shutdown it records every loaded class with its code source.
 * Output directory: the agent argument.
 */
public final class Agent {
    private static volatile PrintStream dispatch;

    private Agent() {
    }

    public static void premain(String args, Instrumentation inst) throws IOException {
        String dir = args == null || args.isEmpty() ? "/tmp" : args;
        dispatch = new PrintStream(new FileOutputStream(dir + "/dispatch.tsv", true), true, StandardCharsets.UTF_8);
        Module base = Object.class.getModule();
        inst.redefineModule(base, Set.of(), Map.of("jdk.internal.org.objectweb.asm", Set.of(Agent.class.getModule())),
                Map.of(), Set.of(), Map.of());
        inst.addTransformer(new Transformer());
        Runtime.getRuntime().addShutdownHook(new Thread(() -> dumpClasses(inst, dir + "/classes.tsv")));
    }

    /** Called from the instrumented ApplicationFilterFactory.createFilterChain(request, wrapper, servlet). */
    public static void chain(Object request, Object servlet) {
        String uri = "?";
        try {
            uri = String.valueOf(request.getClass().getMethod("getRequestURI").invoke(request));
        } catch (ReflectiveOperationException | RuntimeException e) {
            uri = "!" + e.getClass().getName();
        }
        PrintStream out = dispatch;
        if (out != null) {
            out.println(uri + "\t" + (servlet == null ? "null" : servlet.getClass().getName()));
        }
    }

    private static void dumpClasses(Instrumentation inst, String path) {
        try (PrintStream out = new PrintStream(new FileOutputStream(path), false, StandardCharsets.UTF_8)) {
            for (Class<?> c : inst.getAllLoadedClasses()) {
                String where = "";
                try {
                    ProtectionDomain pd = c.getProtectionDomain();
                    CodeSource cs = pd == null ? null : pd.getCodeSource();
                    where = cs == null || cs.getLocation() == null ? "" : cs.getLocation().toString();
                } catch (RuntimeException e) {
                    where = "!";
                }
                out.println(c.getName() + "\t" + where);
            }
        } catch (IOException e) {
            System.err.println("grader probe: " + e);
        }
    }

    private static final class Transformer implements ClassFileTransformer {
        private static final String TARGET = "org/apache/catalina/core/ApplicationFilterFactory";

        @Override
        public byte[] transform(ClassLoader loader, String name, Class<?> redefined, ProtectionDomain pd, byte[] bytes) {
            if (!TARGET.equals(name)) {
                return null;
            }
            try {
                ClassReader reader = new ClassReader(bytes);
                ClassWriter writer = new ClassWriter(reader, 0);
                reader.accept(new ClassVisitor(Opcodes.ASM8, writer) {
                    @Override
                    public MethodVisitor visitMethod(int access, String mname, String desc, String sig, String[] ex) {
                        MethodVisitor mv = super.visitMethod(access, mname, desc, sig, ex);
                        if (!"createFilterChain".equals(mname) || (access & Opcodes.ACC_STATIC) == 0) {
                            return mv;
                        }
                        return new MethodVisitor(Opcodes.ASM8, mv) {
                            @Override
                            public void visitCode() {
                                super.visitCode();
                                super.visitVarInsn(Opcodes.ALOAD, 0);
                                super.visitVarInsn(Opcodes.ALOAD, 2);
                                super.visitMethodInsn(Opcodes.INVOKESTATIC, "grader/probe/Agent", "chain",
                                        "(Ljava/lang/Object;Ljava/lang/Object;)V", false);
                            }

                            @Override
                            public void visitMaxs(int stack, int locals) {
                                super.visitMaxs(Math.max(stack, 2), locals);
                            }
                        };
                    }
                }, 0);
                return writer.toByteArray();
            } catch (RuntimeException e) {
                System.err.println("grader probe: transform failed " + e);
                return null;
            }
        }
    }
}
