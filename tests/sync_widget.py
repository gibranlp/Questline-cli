"""Exercise local Sync controls and confirmations against isolated profiles."""
import fcntl
import json
import os
from pathlib import Path
import shutil
import sqlite3
import subprocess
import tempfile
import unittest
import uuid

ROOT=Path(__file__).resolve().parents[1]
BINARY=os.environ.get('QUESTLINE_TEST_BINARY',str(ROOT/'target/debug/questline'))
CONTROL=os.environ.get('SPECTRUM_SYNC_CONTROL',str(ROOT.parent/'SpectrumOS/scripts/questline-sync-control'))
QML=os.environ.get('SPECTRUM_SYNC_QML',str(ROOT.parent/'SpectrumOS/configs/spectrum-shell'))


class SyncWidgetTests(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory(prefix='questline-sync-widget-')
        self.root=Path(self.temp.name);self.storage=self.root/'profile';self.storage.mkdir()
        self.account=str(uuid.uuid4());self.db=self.storage/'questline.db'
        with sqlite3.connect(self.db) as db:
            db.execute('CREATE TABLE users(id TEXT PRIMARY KEY,username TEXT NOT NULL,class TEXT NOT NULL,level INTEGER DEFAULT 1,xp INTEGER DEFAULT 0,created_at TEXT NOT NULL,specialization TEXT)')
            db.execute('INSERT INTO users VALUES (?,?,?,1,0,?,NULL)',(self.account,'Sync tester','Code Warlock','2026-01-01T00:00:00Z'))
            db.execute('CREATE TABLE settings(key TEXT PRIMARY KEY,value TEXT NOT NULL)')
            db.execute("INSERT INTO settings VALUES ('cloud_sync_enabled','false')")
            db.execute("INSERT INTO settings VALUES ('device_id','sync-widget-device')")
        self.env=dict(os.environ,QUESTLINE_STORAGE_DIR=str(self.storage),XDG_STATE_HOME=str(self.root/'state'),XDG_CACHE_HOME=str(self.root/'cache'),SPECTRUM_QUESTLINE_BINARY=BINARY)
        result=self.call('status');self.assertTrue(result['ok'],result)

    def tearDown(self):self.temp.cleanup()

    def call(self,action,**extra):
        result=subprocess.run([BINARY,'sync-json'],input=json.dumps(dict(action=action,account=self.account,**extra))+'\n',capture_output=True,text=True,env=self.env,timeout=20)
        self.assertEqual(result.returncode,0,result.stderr)
        return json.loads(result.stdout.strip().splitlines()[-1])

    def interactive(self,action,**extra):
        p=subprocess.Popen([BINARY,'sync-json'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,env=self.env)
        self.addCleanup(lambda:p.poll() is None and p.kill())
        p.stdin.write(json.dumps(dict(action=action,account=self.account,**extra))+'\n');p.stdin.flush()
        first=json.loads(p.stdout.readline());return p,first

    def finish(self,p,preview,confirm):
        p.stdin.write(json.dumps(dict(confirm=preview['token']) if confirm else dict(cancel=True))+'\n');p.stdin.flush()
        out,err=p.communicate(timeout=20);self.assertEqual(p.returncode,0,err)
        return json.loads(out.strip().splitlines()[-1])

    def add_link(self,title='Original',url='https://example.com/'):
        request=dict(action='apply')
        process=subprocess.run([BINARY,'favorites-json'],input='{}',env=self.env,capture_output=True,text=True,check=True)
        before=json.loads(process.stdout);after=json.loads(process.stdout)
        after['favorites'].append(dict(id=str(uuid.uuid4()),title=title,url=url,category='uncategorized',clickCount=0))
        request.update(account=self.account,before=before,after=after)
        subprocess.run([BINARY,'favorites-json'],input=json.dumps(request),env=self.env,capture_output=True,text=True,check=True)

    def task(self,identifier,completed=1,old=True,parent=None):
        when='2000-01-01T00:00:00Z' if old else '2999-01-01T00:00:00Z'
        with sqlite3.connect(self.db) as db:
            db.execute('INSERT INTO tasks(id,title,completed,created_at,updated_at,parent_task_id) VALUES (?,?,?,?,?,?)',(identifier,identifier,completed,when,when,parent))

    def test_preferences_persist_and_manual_sync_observes_cloud_toggle(self):
        self.assertTrue(self.call('set_auto_sync',enabled=False)['ok'])
        self.assertFalse(self.call('status')['status']['auto_sync'])
        self.assertFalse(self.call('sync_now')['ok'])
        self.assertTrue(self.call('set_auto_sync',enabled=True)['ok'])
        self.assertTrue(self.call('status')['status']['auto_sync'])

    def test_export_and_backup_do_not_modify_current_collection(self):
        self.add_link();file=self.root/'export.json'
        result=self.call('export_profile',path=str(file));self.assertTrue(result['ok'],result)
        data=json.loads(file.read_text());self.assertEqual(len(data['favorites']),1)
        self.assertEqual(file.stat().st_mode&0o777,0o600)
        self.assertFalse(result['cloud_saved']);self.assertNotIn('transfer_code',result)
        result=self.call('backup');self.assertTrue(result['ok'],result)
        with sqlite3.connect(result['path']) as db:self.assertEqual(db.execute('PRAGMA integrity_check').fetchone()[0],'ok')

    def test_widget_export_saves_to_downloads_without_dialog_and_preserves_previous(self):
        tools=self.root/'tools';tools.mkdir()
        downloads=self.root/'My Downloads'
        resolver=tools/'xdg-user-dir'
        resolver.write_text('#!/bin/sh\nprintf "%s\\n" "$TEST_DOWNLOADS"\n');resolver.chmod(0o755)
        chooser=tools/'zenity'
        chooser.write_text('#!/bin/sh\nexit 99\n');chooser.chmod(0o755)
        env=dict(self.env,PATH=str(tools)+os.pathsep+os.environ['PATH'],TEST_DOWNLOADS=str(downloads))
        saved=[]
        for _ in range(2):
            result=subprocess.run([CONTROL],input=json.dumps(dict(action='export_profile',account=self.account))+'\n',capture_output=True,text=True,env=env,timeout=20)
            self.assertEqual(result.returncode,0,result.stderr)
            value=json.loads(result.stdout.strip().splitlines()[-1]);self.assertTrue(value['ok'],value)
            target=Path(value['path']);self.assertEqual(target.parent,downloads)
            self.assertEqual(target.stat().st_mode&0o777,0o600)
            self.assertIn('favorites',json.loads(target.read_text()));saved.append(target)
        self.assertNotEqual(*saved)
        self.assertTrue(all(p.exists() for p in saved))

    def test_restore_file_can_cancel_then_confirm_and_preserves_local_device(self):
        self.add_link();backup=self.call('backup')['path'];self.add_link('Newer','https://newer.example/')
        p,preview=self.interactive('restore_backup',path=backup)
        self.assertEqual(preview['phase'],'preview');self.assertEqual(preview['incoming']['favorites'],1)
        self.assertTrue(self.finish(p,preview,False)['cancelled'])
        with sqlite3.connect(self.db) as db:self.assertEqual(db.execute('SELECT COUNT(*) FROM favorites').fetchone()[0],2)
        p,preview=self.interactive('restore_backup',path=backup);result=self.finish(p,preview,True)
        self.assertTrue(result['ok'],result);self.assertTrue(result['profile_replaced'])
        self.assertFalse(result['status']['auto_sync'])
        with sqlite3.connect(self.db) as db:
            self.assertEqual(db.execute('SELECT COUNT(*) FROM favorites').fetchone()[0],1)
            self.assertEqual(db.execute("SELECT value FROM settings WHERE key='device_id'").fetchone()[0],'sync-widget-device')
            self.assertTrue(db.execute("SELECT value FROM settings WHERE key='shell_profile_epoch'").fetchone()[0])
        self.assertTrue(list(self.storage.glob('questline_backup_prerestore_*.db')))

    def test_restore_detects_changes_during_preview(self):
        self.add_link();backup=self.call('backup')['path']
        p,preview=self.interactive('restore_backup',path=backup)
        self.add_link('Concurrent edit','https://concurrent.example/')
        result=self.finish(p,preview,True)
        self.assertFalse(result['ok']);self.assertIn('Local data changed',result['error'])
        with sqlite3.connect(self.db) as db:self.assertEqual(db.execute('SELECT COUNT(*) FROM favorites').fetchone()[0],2)

    def test_prune_previews_confirms_and_keeps_incomplete_branch(self):
        self.task('old-root');self.task('old-child',parent='old-root');self.task('protected');self.task('unfinished',0,parent='protected')
        p,preview=self.interactive('prune_tasks',days=30);self.assertEqual(preview['count'],2)
        result=self.finish(p,preview,False);self.assertTrue(result['cancelled'])
        p,preview=self.interactive('prune_tasks',days=30);result=self.finish(p,preview,True)
        self.assertTrue(result['ok'],result);self.assertTrue(Path(result['backup']).is_file())
        with sqlite3.connect(self.db) as db:
            self.assertEqual(db.execute('SELECT COUNT(*) FROM tasks').fetchone()[0],2)
            self.assertEqual(db.execute("SELECT COUNT(*) FROM sync_log WHERE entity_type='task' AND operation='delete'").fetchone()[0],2)

    def test_prune_rejects_changed_preview(self):
        self.task('old')
        p,preview=self.interactive('prune_tasks',days=30)
        with sqlite3.connect(self.db) as db:db.execute("UPDATE tasks SET completed=0 WHERE id='old'")
        result=self.finish(p,preview,True);self.assertFalse(result['ok']);self.assertIn('Tasks changed',result['error'])

    def test_restore_rejects_corrupt_file_and_active_terminal(self):
        broken=self.root/'broken.db';broken.write_bytes(b'SQLite format 3\0'+b'broken')
        self.assertFalse(self.call('restore_backup',path=str(broken))['ok'])
        with (self.storage/'desktop-tui.lock').open('a') as lock:
            fcntl.flock(lock,fcntl.LOCK_SH)
            result=self.call('prune_tasks',days=30)
        self.assertFalse(result['ok']);self.assertIn('Close the Questline terminal',result['error'])
        self.assertFalse(self.call('restore_identity',transfer_code='not a transfer code')['ok'])

    def test_wrapper_streams_preview_and_preserves_drafts_before_restore(self):
        self.add_link();backup=self.call('backup')['path'];self.add_link('Second','https://second.example/')
        state=Path(self.env['XDG_STATE_HOME'])/'spectrumos/questline-widget.json';state.parent.mkdir(parents=True);state.write_text('{"version":1,"drafts":{"private-draft":"keep me"}}')
        p=subprocess.Popen([CONTROL],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,env=self.env)
        self.addCleanup(lambda:p.poll() is None and p.kill())
        p.stdin.write(json.dumps(dict(action='restore_backup',account=self.account,path=backup))+'\n');p.stdin.flush()
        preview=json.loads(p.stdout.readline());result=self.finish(p,preview,True)
        self.assertTrue(result['profile_replaced']);copies=list(state.parent.glob('questline-draft-backups/*.json'))
        self.assertEqual(len(copies),1);self.assertEqual(copies[0].read_text(),state.read_text());self.assertEqual(copies[0].stat().st_mode&0o777,0o600)

    @unittest.skipUnless(os.environ.get('QUESTLINE_SYNC_TEST_API'), 'Disposable sync API not requested')
    def test_encrypted_cloud_export_current_identity_restore_and_second_pc(self):
        server=os.environ['QUESTLINE_SYNC_TEST_API']
        self.assertTrue(server.startswith('http://127.0.0.1:'))
        config='server_url = '+json.dumps(server)+'\nsync_enabled = true\nauto_sync = false\nsync_interval_minutes = 5\n'
        (self.storage/'config.toml').write_text(config)
        with sqlite3.connect(self.db) as db:db.execute("UPDATE settings SET value=? WHERE key='device_id'",(str(uuid.uuid4()),))
        self.call('set_cloud_sync',enabled=True);self.add_link('Encrypted control test')
        exported=self.call('export_profile',path=str(self.root/'cloud-profile.json'))
        self.assertTrue(exported.get('cloud_saved'),exported)
        self.assertIn('transfer_code',exported)
        self.add_link('Unpublished local edit','https://unsent.example/')
        p,preview=self.interactive('restore_cloud')
        self.assertEqual(preview.get('phase'),'preview',preview);self.assertEqual(preview['incoming']['favorites'],1)
        self.assertTrue(self.finish(p,preview,False)['cancelled'])
        other=SyncWidgetTests();other.setUp();self.addCleanup(other.tearDown)
        (other.storage/'config.toml').write_text(config)
        with sqlite3.connect(other.db) as db:db.execute("UPDATE settings SET value=? WHERE key='device_id'",(str(uuid.uuid4()),))
        other.call('set_cloud_sync',enabled=True)
        p,preview=other.interactive('restore_identity',transfer_code=exported['transfer_code'])
        self.assertEqual(preview.get('phase'),'preview',preview);self.assertEqual(preview['incoming']['favorites'],1)
        result=other.finish(p,preview,True)
        self.assertTrue(result['ok'],result);self.assertEqual(result['status']['account'],self.account)
        self.assertFalse(result['status']['auto_sync'])
        other.account=self.account
        self.assertTrue(other.call('sync_now')['ok'])

    @unittest.skipUnless(shutil.which('qs'),'Quickshell unavailable')
    def test_rendered_sync_controls_use_real_backend_and_confirm_cancel(self):
        directory=self.root/'backend';directory.mkdir();shutil.copy2(CONTROL,directory/'questline-sync-control')
        shutil.copy2(Path(QML).parents[1]/'scripts/questline-widget-state',directory/'questline-widget-state')
        runtime=self.root/'runtime';runtime.mkdir(mode=0o700)
        qml=self.root/'SyncTest.qml'
        qml.write_text(r'''import QtQuick
import QtQuick.Controls
import QtTest
import Quickshell
import "file:__QML__" as Shell
ApplicationWindow {
 id:window;width:900;height:1000;visible:true
 Shell.QuestlineWorkspace{id:workspace;backendDir:"__BACKEND__"}
 Shell.QuestlineSync {
  id:panel;anchors.fill:parent;active:true;workspace:workspace;backendDir:"__BACKEND__"
  theme:({colors:{foreground:"#eeeeee",background:"#111111",muted:"#aaaaaa",accent:"#a060ff",surface_alt:"#303030"},fontFamily:"Sans",fontSize:13})
 }
 TestCase {
  name:"SyncControls";when:window.visible
  function button(name){return findChild(panel,"questline-sync-"+name);}
  function test_control_round_trip_and_cancel(){
   tryVerify(()=>!panel.busy && panel.state.available,5000);
   verify(!panel.state.cloud_enabled);verify(panel.state.auto_sync);
   mouseClick(button("auto"));tryVerify(()=>!panel.busy && !panel.state.auto_sync,5000);compare(panel.error,"");
   mouseClick(button("auto"));tryVerify(()=>!panel.busy && panel.state.auto_sync,5000);
   mouseClick(button("backup"));tryVerify(()=>!panel.busy && panel.message.includes("backup"),5000);verify(panel.message.includes("backup"));compare(panel.error,"");
   panel.run("prune_tasks",{days:30});tryVerify(()=>panel.preview!==null,5000);verify(panel.busy);verify(!button("backup").enabled);
   mouseClick(button("cancel"));tryVerify(()=>!panel.busy,5000);verify(panel.message.includes("canceled"));
   verify(panel.preview===null);compare(workspace.externalActions,0);
   workspace.update("stale",{action:"create_scroll",title:"Old draft"},false,"unfinished",false);
   workspace.bindProfile("restored-profile:epoch");tryVerify(()=>Object.keys(workspace.drafts).length===0,5000);compare(workspace.activeTab,"Sync");
   const box=button("transfer-input");box.text="Do not persist this code";verify(!JSON.stringify(workspace.snapshot()).includes(box.text));panel.active=false;compare(box.text,"");compare(panel.transferCode,"");
   console.log("SYNC CONTROLS PASSED");
  }
  onCompletedChanged:if(completed){console.log("SYNC CHECK COUNTS",qtest_results.passCount,qtest_results.failCount);Qt.quit();}
 }
 Timer{interval:18000;running:true;onTriggered:{console.log("SYNC CONTROLS TIMEOUT",panel.error,panel.message);Qt.quit();}}
}'''.replace('__QML__',QML).replace('__BACKEND__',str(directory)))
        result=subprocess.run(['qs','--path',str(qml),'--no-color'],capture_output=True,text=True,env=dict(self.env,QT_QPA_PLATFORM='offscreen',QT_QUICK_BACKEND='software',XDG_RUNTIME_DIR=str(runtime)),timeout=25)
        self.assertEqual(result.returncode,0,result.stdout+result.stderr)
        self.assertIn('SYNC CONTROLS PASSED',result.stdout+result.stderr)
        copies=list((Path(self.env['XDG_STATE_HOME'])/'spectrumos/questline-draft-backups').glob('*.json'))
        self.assertTrue(copies)
        self.assertTrue(any('Old draft' in p.read_text() for p in copies))


if __name__=='__main__':unittest.main()
