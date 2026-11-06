"""Opt-in real LWJGL/GLFW loader spike using only the installed official runtime/JARs."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
text = Path(sys.argv[1]).read_text(encoding="utf-8")
snapshot = json.loads(text[text.index('{'):])
base = Path(sys.argv[2]) / "data"
classpath = os.pathsep.join(str(base / entry) for entry in snapshot["classpath"])
source = '''import org.lwjgl.system.MemoryUtil;
import org.lwjgl.glfw.GLFW;
import org.lwjgl.system.MemoryStack;
import org.lwjgl.util.freetype.FreeType;
public class NativeProbe {
 public static void main(String[] args) {
  var buffer=MemoryUtil.memAlloc(16); MemoryUtil.memFree(buffer);
  int[] major={0},minor={0},patch={0};
  GLFW.glfwGetVersion(major,minor,patch);
  if(major[0]!=3) throw new IllegalStateException("Unexpected GLFW");
  try(var stack=MemoryStack.stackPush()) {
   var library=stack.mallocPointer(1);
   if(FreeType.FT_Init_FreeType(library)!=0) throw new IllegalStateException("FreeType init");
   FreeType.FT_Done_FreeType(library.get(0));
  }
  System.out.println("LWJGL, GLFW and FreeType native loading passed: "+System.getProperty("os.arch"));
 }
}'''
environment = os.environ.copy()
for name in ["JAVA_TOOL_OPTIONS", "_JAVA_OPTIONS", "JDK_JAVA_OPTIONS", "CLASSPATH", "LD_PRELOAD", "LD_LIBRARY_PATH", "DYLD_INSERT_LIBRARIES", "DYLD_LIBRARY_PATH", "DYLD_FRAMEWORK_PATH"]:
    environment.pop(name, None)
with tempfile.TemporaryDirectory(prefix="launcher-native-") as folder:
    path = Path(folder) / "NativeProbe.java"
    path.write_text(source, encoding="utf-8")
    executable=snapshot["javaExecutable"]
    if sys.platform=="win32" and executable.startswith("\\\\?\\"):
        raise RuntimeError("verbatim JVM execution path must be normalized by backend")
    arguments = [executable]
    if sys.platform == "darwin":
        arguments.append("-XstartOnFirstThread")
    arguments += ["-Dorg.lwjgl.system.SharedLibraryExtractPath=" + str(Path(folder) / "natives"), "--class-path", classpath, "--source", "21", str(path)]
    subprocess.run(arguments, check=True, env=environment, timeout=30)
