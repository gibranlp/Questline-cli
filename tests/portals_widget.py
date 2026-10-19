"""Isolated integration checks against the compiled CLI and SpectrumOS adapter."""
import importlib.machinery
import importlib.util
import json
import os
from pathlib import Path
import sqlite3
import shutil
import subprocess
import tempfile
import unittest
from unittest.mock import patch
import uuid

ROOT = Path(__file__).resolve().parents[1]
ADAPTER = Path(os.environ.get('SPECTRUM_FAVORITES_ADAPTER', str(ROOT.parent/'SpectrumOS/scripts/favorites-adapter')))
loader = importlib.machinery.SourceFileLoader('favorites_integration', str(ADAPTER))
spec = importlib.util.spec_from_loader(loader.name, loader)
backend = importlib.util.module_from_spec(spec)
loader.exec_module(backend)
BINARY = os.environ.get('QUESTLINE_TEST_BINARY', str(ROOT/'target/debug/questline'))


class PortalWidgetTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix='questline-portals-widget-')
        self.root = Path(self.tmp.name)
        self.storage = self.root/'questline'
        self.storage.mkdir()
        self.db = self.storage/'questline.db'
        with sqlite3.connect(self.db) as connection:
            connection.execute('CREATE TABLE users (id TEXT PRIMARY KEY, username TEXT NOT NULL, class TEXT NOT NULL, level INTEGER NOT NULL DEFAULT 1, xp INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL, specialization TEXT)')
            connection.execute('INSERT INTO users VALUES (?,?,?,1,0,?,NULL)', (str(uuid.uuid4()), 'Explorer', 'Code Warlock', '2026-10-19T00:00:00Z'))
        self.env = patch.dict(os.environ, QUESTLINE_STORAGE_DIR=str(self.storage), XDG_STATE_HOME=str(self.root/'state'), XDG_CACHE_HOME=str(self.root/'cache'), SPECTRUM_FAVORITES_QUESTLINE=BINARY)
        self.env.start()

    def tearDown(self):
        self.env.stop()
        self.tmp.cleanup()

    def cli(self, *args):
        result = subprocess.run([BINARY, 'portals', *args], capture_output=True, text=True, timeout=15)
        self.assertEqual(result.returncode, 0, result.stderr)
        return result.stdout

    def test_widget_creates_moves_and_deletes_through_cli(self):
        state = backend.main('status', {})['state']
        self.assertIn('questlineAccount', state)
        backend.main('view-save', {'view': dict(category='top', search='', editor='category', draft=dict(id='work', name='Work'), baseVersion=None)})
        backend.main('view-save', {'view': dict(category='work', search='', editor='favorite', draft=dict(id='one', title='Private portal', url='https://example.com/secret?q=1#two', category='work'), baseVersion=None)})
        self.assertIn('Private portal', self.cli('list'))
        self.cli('edit', 'one', 'Edited in Questline', 'https://example.com/changed', 'Uncategorized')
        state = backend.main('status', {})['state']
        self.assertEqual(state['favorites'][0]['title'], 'Edited in Questline')
        self.assertEqual(state['favorites'][0]['category'], 'uncategorized')
        backend.main('favorite-delete', {'id': 'one'})
        self.assertNotIn('Edited in Questline', self.cli('list'))

    def test_migration_preserves_local_view_and_remaps_ids(self):
        state = backend.initial()
        state['categories'].append(dict(id='old-work', name='Work'))
        state['favorites'].append(dict(id='old-link', title='Existing link', url='https://example.com', category='old-work', clickCount=12))
        state['view'] = dict(category='old-work', search='local search', editor='', draft={})
        backend.atomic(backend.directory()/'favorites.json', json.dumps(state).encode())
        first = backend.main('status', {})['state']
        second = backend.main('status', {})['state']
        self.assertEqual(first['favorites'], second['favorites'])
        self.assertEqual(first['favorites'][0]['clickCount'], 12)
        self.assertEqual(first['view']['search'], 'local search')
        self.assertEqual(first['view']['category'], first['favorites'][0]['category'])
        self.assertIn('Existing link', self.cli('list'))
        self.assertEqual((backend.directory()/'favorites.json').stat().st_mode & 0o777, 0o600)

    def test_remote_edit_does_not_get_overwritten_by_open_editor(self):
        self.cli('add', 'Original', 'https://example.com')
        state = backend.main('status', {})['state']
        link = state['favorites'][0]
        version = state['_versions']['favorite:'+link['id']]
        self.cli('edit', link['id'], 'Remote edit', 'https://remote.example')
        response = backend.main('view-save', {'view': dict(category='top', search='', editor='favorite', draft=dict(id=link['id'], title='Unfinished local edit', url=link['url'], category=link['category']), baseVersion=version)})
        self.assertIn('changed on another device', response['draftError'])
        self.assertEqual(response['state']['favorites'][0]['title'], 'Remote edit')
        self.assertEqual(backend.read()['view']['draft']['title'], 'Unfinished local edit')

    def test_profile_switch_clears_editor_and_never_copies_previous_links(self):
        self.cli('add', 'Previous account link', 'https://previous.example')
        state = backend.main('status', {})['state']
        link = state['favorites'][0]
        local = backend.read()
        local['view'] = dict(category='top', search='', editor='favorite', draft=link)
        backend.atomic(backend.directory()/'favorites.json', json.dumps(local).encode())
        other = self.root/'other'
        other.mkdir()
        import shutil
        shutil.copy2(self.db, other/'questline.db')
        with sqlite3.connect(other/'questline.db') as connection:
            connection.execute('UPDATE users SET id=?', (str(uuid.uuid4()),))
            connection.execute('DELETE FROM favorites')
            connection.execute('DELETE FROM favorite_visits')
            connection.execute('DELETE FROM favorite_categories')
        with patch.dict(os.environ, QUESTLINE_STORAGE_DIR=str(other)):
            changed = backend.main('status', {})['state']
        self.assertEqual(changed['favorites'], [])
        self.assertEqual(changed['view']['editor'], '')
        self.assertNotEqual(changed['questlineAccount'], state['questlineAccount'])

    def test_repeated_editor_autosaves_update_version_and_preserve_draft(self):
        payload = dict(view=dict(category='top', search='', editor='favorite', draft=dict(id='autosaved', title='First', url='https://example.com', category='uncategorized'), baseVersion=None))
        first = backend.main('view-save', payload)
        self.assertEqual(first['draftError'], '')
        payload['view']['baseVersion'] = first['state']['view']['baseVersion']
        payload['view']['draft']['title'] = 'Second'
        second = backend.main('view-save', payload)
        self.assertEqual(second['draftError'], '')
        self.assertEqual(second['state']['favorites'][0]['title'], 'Second')
        self.assertNotEqual(first['state']['view']['baseVersion'], second['state']['view']['baseVersion'])

    def test_closed_widget_refresh_keeps_unfinished_draft_device_local(self):
        response=backend.main('view-save', dict(view=dict(category='top', search='idea', editor='favorite', draft=dict(id='draft', title='Idea', url='https://', category='uncategorized'), baseVersion=None)))
        self.assertTrue(response['draftError'])
        state=backend.main('status', {})['state']
        self.assertEqual(state['favorites'], [])
        self.assertEqual(state['view']['draft']['url'], 'https://')
        with sqlite3.connect(self.db) as connection:
            self.assertEqual(connection.execute('SELECT COUNT(*) FROM favorites').fetchone()[0], 0)


    @unittest.skipUnless(shutil.which('qs'), 'Quickshell is unavailable')
    def test_real_widget_queue_remote_refresh_and_stale_editor(self):
        shell = Path(os.environ.get('SPECTRUM_FAVORITES_QML', str(ADAPTER.parent.parent/'configs/spectrum-shell')))
        self.assertTrue((shell/'FavoritesManager.qml').is_file())
        wrapper = self.root/'adapter'
        wrapper.write_text("#!/usr/bin/env python3\nimport runpy,json,sys\nm=runpy.run_path("+repr(str(ADAPTER))+")\nm['fetch_icon'].__globals__['fetch_icon']=lambda url:None\ntry:\n print(json.dumps(m['main'](sys.argv[1],json.loads(sys.stdin.readline() or '{}'))))\nexcept Exception as exc:\n print(json.dumps(dict(ok=False,error=str(exc))));sys.exit(1)\n")
        wrapper.chmod(0o755)
        qml = self.root/'Test.qml'
        qml.write_text(r'''import QtQuick
import Quickshell
import Quickshell.Io
import "file:__SHELL__" as Shell
Scope {
    property int step:0
    Shell.FavoritesManager {id:manager;active:true;theme:({backend:function(name){return "__WRAPPER__";}})}
    Process {id:remote}
    Timer {interval:50;running:true;repeat:true;onTriggered:{
        if(!manager.ready)return;
        if(step===0){
            manager.editFavorite(null);manager.draft({title:"First",url:"https://example.com/"});manager.persist();
            manager.draft({title:"Second"});manager.persist();manager.draft({title:"Third"});manager.persist();step=1;
        }else if(step===1 && !manager.busy && !manager.queue.length && !manager.pendingView){
            if(manager.error || manager.draftError || manager.favorites.length!==1 || manager.favorites[0].title!=="Third"){console.log("PORTALS FAIL autosave",manager.error,manager.draftError,JSON.stringify(manager.favorites));Qt.quit();return;}
            remote.exec(["__BINARY__","portals","edit",manager.favorites[0].id,"Changed remotely","https://remote.example/"]);step=2;
        }else if(step===2 && !remote.running && manager.favorites[0].title==="Changed remotely"){
            if(manager.view.draft.title!=="Third"){console.log("PORTALS FAIL draft");Qt.quit();return;}
            manager.draft({title:"Stale fourth"});manager.persist();step=3;
        }else if(step===3 && !manager.busy && !manager.queue.length && manager.draftError){
            if(manager.favorites[0].title==="Changed remotely" && manager.view.draft.title==="Stale fourth" && manager.draftError.includes("changed on another device"))console.log("PORTALS WIDGET SYNC PASSED");
            else console.log("PORTALS FAIL conflict",manager.draftError);
            Qt.quit();
        }
    }}
    Timer {interval:12000;running:true;onTriggered:{console.log("PORTALS FAIL timeout",step,manager.error,manager.draftError);Qt.quit();}}
}
'''.replace('__SHELL__', str(shell)).replace('__WRAPPER__', str(wrapper)).replace('__BINARY__', BINARY))
        runtime=self.root/'runtime';runtime.mkdir(mode=0o700)
        result=subprocess.run(['qs','--path',str(qml),'--no-color'],capture_output=True,text=True,timeout=20,env=dict(os.environ,QT_QPA_PLATFORM='offscreen',QT_QUICK_BACKEND='software',XDG_RUNTIME_DIR=str(runtime)))
        self.assertEqual(result.returncode,0,result.stdout+result.stderr)
        self.assertIn('PORTALS WIDGET SYNC PASSED',result.stdout+result.stderr)


if __name__ == '__main__':
    unittest.main()
