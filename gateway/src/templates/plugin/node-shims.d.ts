declare module "node:fs" {
  export function appendFileSync(path: string, data: string, encoding: "utf-8"): void
  export function existsSync(path: string): boolean
  export function mkdirSync(path: string, options: { recursive: boolean }): void
  export function readFileSync(path: string, encoding: "utf-8"): string
  export function readdirSync(path: string): string[]
}

declare module "node:path" {
  export function join(...paths: string[]): string
}
