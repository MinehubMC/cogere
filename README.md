# cogere - cache, assembler and repository for minecraft plugins

TODO:
- [ ] Authentication
  - [ ] API keys with a description (used by developers, admins and machines)
  - [ ] RBAC
  - [ ] Guest/Anonymous access for specific resources
  - [ ] Groups
- [ ] Act as a maven repository
  - [ ] Upload artifact
  - [ ] Get artifact
  - [ ] Delete artifact
  - [ ] Artifact browser
  - [ ] Quota
  - [ ] Restrict artifact to a specific group
- [ ] Plugin repository
  - [ ] Quota
  - [ ] Version management
  - [ ] Restricted to group
- [ ] Assembler
  - [ ] Assemble a .zip file from a "lockfile", similar to package.json dependencies.json
    - [ ] For example it could request live.minehub:PolarPaper:1.21.11.* or something similiar
  - [ ] External plugin sources
    - [ ] Background job to check for new releases
    - [ ] Only add entries to database and do not download files
    - [ ] Cache plugins only as requested. Not sure how I want to handle this as that could take some time.
