import createConfig from "./";

export default createConfig({
  format: ["esm"],
  entry: ["worker.ts"],
  platform: "browser",
});
