import fs from "node:fs";
import { randomUUID } from "node:crypto";

const tag = process.env.GITHUB_REF_NAME;
if (!/^v\d+\.\d+\.\d+$/.test(tag || "")) {
  throw new Error("Release notes require a stable version tag.");
}
const body = fs.readFileSync(`docs/release-notes/${tag}.md`, "utf8").trim();
if (!body) throw new Error(`Release notes for ${tag} are empty.`);
if (process.env.GITHUB_OUTPUT) {
  const delimiter = `notes_${randomUUID()}`;
  fs.appendFileSync(
    process.env.GITHUB_OUTPUT,
    `body<<${delimiter}\n${body}\n${delimiter}\n`,
  );
} else {
  console.log(body);
}
