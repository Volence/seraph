export function formatTags(tags: string[]): string {
  return tags.map((t) => t.toLowerCase()).join(", ");
}
