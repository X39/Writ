import init, { WritVm } from "./pkg/writ_runtime_wasm.js?v=1";

const output = document.querySelector("#output");
const lines = [];
const write = (line) => {
  lines.push(line);
  output.textContent = lines.join("\n");
};

await init();
const vm = new WritVm();
vm.setLogCallback((level, message) => write(`[${level}] ${message}`));

vm.setHostCallback((request) => {
  if (request.type === "externCall" && request.name === "host_add_one") {
    const input = BigInt(request.args[0].value);
    write(`sync host_add_one(${input})`);
    return { kind: "value", value: { kind: "int", value: String(input + 1n) } };
  }

  if (request.type === "externCall" && request.name === "host_wait") {
    write(`deferred: ${request.args[0].value}`);
    setTimeout(() => {
      vm.resolveRequest(request.id, {
        kind: "value",
        value: { kind: "string", value: "Browser host" },
      });
      write(`resolved request ${request.id}`);
    }, 1000);
    return { kind: "deferred" };
  }

  return { kind: "error", message: `unsupported host request: ${request.type}` };
});

const moduleBytes = new Uint8Array(await (await fetch("./sample.writc")).arrayBuffer());
vm.loadModule(moduleBytes);
const task = vm.spawn("main", []);
write(`spawned task ${task.index}:${task.generation}`);
let lastStatus = "";

function drive(now) {
  try {
    const result = vm.tick(now / 1000, 2_000);
    const state = vm.taskState(task);
    const status = `${result.status}/${state}`;
    if (status !== lastStatus) {
      write(`tick: ${result.status}; task: ${state}`);
      lastStatus = status;
    }

    if (state === "completed") {
      write(`return value: ${JSON.stringify(vm.returnValue(task))}`);
      return;
    }
    if (state === "cancelled") {
      write(vm.taskError(task) ?? vm.lastError() ?? "unknown runtime error");
      return;
    }
    requestAnimationFrame(drive);
  } catch (error) {
    write(`boundary error: ${error.message ?? error}`);
  }
}

requestAnimationFrame(drive);
