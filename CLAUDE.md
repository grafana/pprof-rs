# Claude Instructions for pprof-rs

## Pull Request Titles

When creating pull requests, always use conventional commit format in the title. The PR title will become the commit message when squash-merged, so it must follow this format:

### Format

```
<type>[optional !]: <description>
```

### Allowed Types

- **`feat:`** - New feature (triggers **minor** version bump, e.g., 0.1.0 → 0.2.0)
  - Example: `feat: add support for custom profiling intervals`

- **`fix:`** - Bug fix (triggers **patch** version bump, e.g., 0.1.0 → 0.1.1)
  - Example: `fix: resolve SIGPROF race condition in profiler`

- **`ci:`** - CI/CD changes (triggers **patch** version bump)
  - Example: `ci: add release-please workflow`

- **`chore:`** - Maintenance tasks (triggers **patch** version bump)
  - Example: `chore: update dependencies`

### Breaking Changes

Add `!` after the type for breaking changes (triggers **major** version bump, e.g., 0.1.0 → 1.0.0):
- `feat!: redesign profiler API`
- `fix!: change default profiling frequency`

### Guidelines

1. **Keep titles concise** - Under 70 characters
2. **Use imperative mood** - "add feature" not "added feature"
3. **No period at the end** - `feat: add feature` not `feat: add feature.`
4. **Lowercase after colon** - `feat: add feature` not `feat: Add feature`
5. **PR title validation** - CI will enforce these rules automatically

### Why This Matters

- PR titles become commit messages when squash-merged
- [release-please](https://github.com/googleapis/release-please) uses these to:
  - Automatically generate changelogs
  - Determine version bumps
  - Create release PRs
- Consistent format makes the git history readable and professional
