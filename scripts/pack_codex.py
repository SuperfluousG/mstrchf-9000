import json, pathlib, sys, zipfile, yaml

root = pathlib.Path(__file__).resolve().parent.parent
plugin = root / 'codex-plugin'
release = root / 'release'
release.mkdir(exist_ok=True)
manifest = json.loads((plugin / 'plugin.json').read_text(encoding='utf-8'))
compat = json.loads((plugin / '.codex-plugin/plugin.json').read_text(encoding='utf-8'))
assert manifest['$schema'] == 'https://agent-plugins.org/schemas/1.0.0/plugin.schema.json'
assert (manifest['name'], manifest['version']) == (compat['name'], compat['version'])
assert manifest['extensions']['com.openai']['interface'] == compat['interface']
assert len(compat['interface']['shortDescription']) <= 30
for field in ('logo', 'composerIcon'):
    icon = (plugin / compat['interface'][field]).resolve()
    assert icon.is_relative_to(plugin) and icon.is_file()
for file in ('mcp.json', '.mcp.json'):
    config = json.loads((plugin / file).read_text(encoding='utf-8'))
    assert config['mcpServers']['mstrchf_9000']['args'] == ['${PLUGIN_ROOT}/server/index.mjs']
skills = list((plugin / 'skills').glob('*/SKILL.md'))
assert len(skills) == 3
for skill in skills:
    header = skill.read_text(encoding='utf-8').split('---', 2)[1]
    meta = yaml.safe_load(header)
    assert meta['name'] == skill.parent.name and isinstance(meta['description'], str) and meta['description'].strip()
for required in ('server/index.mjs', 'app/planner.html', 'assets/icon.svg', 'catalogue/ingredients.json'):
    assert (plugin / required).is_file(), required
assert len(list((plugin / 'app/photos').glob('*.webp'))) >= 390
files = sorted(f for f in plugin.rglob('*') if f.is_file())
assert all(not f.is_symlink() and 'node_modules' not in f.parts and '.env' not in f.name and 'kitchen.json' != f.name for f in files)
portable = release / f'mstrchf-9000-chatgpt-{manifest["version"]}.zip'
with zipfile.ZipFile(portable, 'w', zipfile.ZIP_DEFLATED) as archive:
    for file in files: archive.write(file, file.relative_to(plugin).as_posix())
marketplace = {'name': 'mstrchf-local', 'interface': {'displayName': 'MSTRCHF local'}, 'plugins': [{'name': 'mstrchf-9000', 'source': {'source': 'local', 'path': './plugin'}, 'policy': {'installation': 'AVAILABLE', 'authentication': 'ON_INSTALL'}, 'category': 'Productivity'}]}
bundle = release / f'mstrchf-9000-codex-{manifest["version"]}-install.zip'
prefix = 'mstrchf-9000-codex/'
with zipfile.ZipFile(bundle, 'w', zipfile.ZIP_DEFLATED) as archive:
    archive.writestr(prefix + '.agents/plugins/marketplace.json', json.dumps(marketplace, indent=2))
    archive.writestr(prefix + 'GET_STARTED.md', '# MSTRCHF 9000 for Codex\n\nRequires Node.js 20+ and Codex desktop. Register this unpacked folder:\n\n```sh\ncodex plugin marketplace add "path/to/mstrchf-9000-codex"\ncodex plugin add mstrchf-9000@mstrchf-local\n```\n\nOpen a new Codex chat and ask to open MSTRCHF 9000. Kitchen storage is outside this folder; see plugin/README.md.\n')
    for file in files: archive.write(file, prefix + 'plugin/' + file.relative_to(plugin).as_posix())
for path in (portable, bundle):
    with zipfile.ZipFile(path) as archive:
        assert archive.testzip() is None
        assert all(not n.startswith('/') and '..' not in pathlib.PurePosixPath(n).parts for n in archive.namelist())
    print(json.dumps({'archive': str(path), 'bytes': path.stat().st_size, 'skills': len(skills), 'files': len(files)}))
