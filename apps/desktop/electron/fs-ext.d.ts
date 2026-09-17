declare module "fs-ext" {
  const fsExt: {
    flockSync(fd: number, flags: "exnb"): void;
  };
  export default fsExt;
}
