export function slugify(text: string): string {
  return text
    .toLowerCase()
    .replace(/[^a-z0-9 _-]+/g, "")
    .trim()
    .replace(/\s+/g, "-");
}
