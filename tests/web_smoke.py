# 从仓库根目录运行：python3 tests/web_smoke.py；先执行 ./build.sh debug build config.example.toml。
# 使用临时数据库、临时端口和示例密码，不访问实际账本；输出 curl 样例到 docs/blueprints/web-api-curl.txt。
import base64, json, pathlib, shlex, socket, sqlite3, subprocess, tempfile, time
root=pathlib.Path.cwd()
report=[]
with tempfile.TemporaryDirectory(prefix='micjz-web-smoke-') as tmp:
    tmp=pathlib.Path(tmp)
    old=subprocess.check_output(['git','show','e8725ae:src/db.rs'],text=True)
    schema=old.split('pub const SCHEMA: &str = "',1)[1].split('";',1)[0]
    old_db=tmp/'v5.db'
    with sqlite3.connect(old_db) as conn:
        conn.executescript(schema+(root/'src/seed.sql').read_text()+'PRAGMA user_version=5;')
        conn.execute("INSERT INTO entries(id,kind,amount_cents,category_id,account_id,occurred_at,created_at) VALUES ('preserved','expense',1234,1,1,'2026-11-17T00:00:00Z','2026-11-17T00:00:00Z')")
        conn.execute('UPDATE settings SET cycle_day=25,budget_cents=50000')
    conf=tmp/'old.toml';conf.write_text(f'db_path = "{old_db}"\n')
    for mode in ['--check','--apply']:
        result=subprocess.check_output(['python3','deploy/db/convert.py','--config',str(conf),mode],text=True)
        report.extend(['数据库转换 '+mode,result])
    with sqlite3.connect(old_db) as conn:
        assert conn.execute('PRAGMA user_version').fetchone()[0]==6
        assert conn.execute("SELECT amount_cents FROM entries WHERE id='preserved'").fetchone()[0]==1234
        assert conn.execute('SELECT cycle_day,budget_cents FROM settings').fetchone()==(25,50000)
        assert conn.execute('SELECT brand FROM accounts').fetchall()==[(None,),(None,)]
        assert not conn.execute('PRAGMA foreign_key_check').fetchall()
    assert list(tmp.glob('v5.db.v5-*.bak'))
    report.append('PASS：v5 → v6 保留账目、设置和账户，生成备份，外键检查通过。\n')
    with socket.socket() as sock:
        sock.bind(('127.0.0.1',0));port=sock.getsockname()[1]
    base=f'http://127.0.0.1:{port}'
    config=tmp/'config.toml'; config.write_text(f'listen = "127.0.0.1:{port}"\ntoken = "web-api-smoke"\ndb_path = "{tmp / "app.db"}"\n')
    log=(tmp/'server.log').open('w')
    proc=subprocess.Popen(['target/debug/micjz',str(config)],stdout=log,stderr=log)
    def curl(method,path,data=None,expected=200,auth=True,extra=None,binary=False):
        args=['curl','--silent','--show-error','--request',method,base+path]
        if auth:
            args+=['--cookie',str(tmp/'cookie')] if path.startswith('/api/') else ['--header','Authorization: Bearer web-api-smoke']
        if data is not None: args+=['--json',json.dumps(data,ensure_ascii=False,separators=(',',':'))]
        if extra: args+=extra
        headers=tmp/'headers';body=tmp/'body'
        result=subprocess.check_output(args+['--dump-header',str(headers),'--output',str(body),'--write-out','%{http_code}'],text=True)
        assert int(result)==expected,(method,path,result,body.read_bytes())
        content=body.read_bytes()
        report.extend(['> '+shlex.join(args),headers.read_text().strip(),f'[{len(content)} bytes, PNG 截图]' if binary else content.decode('utf-8'),''])
        if binary: return content
        return json.loads(content) if content and not path.endswith('.csv') and path.startswith('/api/') else content
    try:
        for _ in range(100):
            if subprocess.run(['curl','--silent',base+'/'],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL).returncode==0: break
            if proc.poll() is not None: raise RuntimeError((tmp/'server.log').read_text())
            time.sleep(.05)
        curl('GET','/',auth=False)
        curl('GET','/icon.svg',auth=False)
        curl('GET','/api/catalog',auth=False,expected=401)
        curl('GET','/api/catalog',expected=401,auth=False,extra=['--header','Authorization: Bearer web-api-smoke'])
        curl('POST','/api/login',{'username':'dzmfg','password':'smoke-password'},expected=409,auth=False)
        result=subprocess.check_output(['target/debug/micjz',str(config),'password'],input='smoke-password\nsmoke-password\n',text=True)
        report.extend(['> micjz /tmp/config.toml password（管道输入两次示例密码）',result])
        curl('POST','/api/login',{'username':'dzmfg','password':'smoke-password'},expected=204,auth=False,extra=['--cookie-jar',str(tmp/'cookie')])
        c=curl('GET','/api/catalog',auth=False,extra=['--cookie',str(tmp/'cookie')])
        curl('GET','/api/settings');curl('GET','/api/balances')
        q='/api/entries?from=2026-01-01T00:00:00Z&to=2027-01-01T00:00:00Z'
        curl('GET',q)
        e={'kind':'expense','amount_cents':3250,'category_id':next(c['id'] for c in c['categories'] if c['kind']=='expense'),'account_id':c['accounts'][0]['id'],'to_account_id':None,'merchant':None,'occurred_at':'2026-11-17T12:30:00.123+08:00','note':'午饭','tag_ids':[]}
        expense=curl('POST','/api/entries',e,expected=201)
        e['kind']='income'; e['category_id']=next(c['id'] for c in c['categories'] if c['kind']=='income')
        income=curl('POST','/api/entries',e,expected=201)
        e['kind']='transfer';e['category_id']=None;e['to_account_id']=c['accounts'][1]['id']
        transfer=curl('POST','/api/entries',e,expected=201)
        curl('PATCH','/api/entries/'+expense['id'],{'note':None,'merchant':'  瑞幸  '})
        curl('GET',q);curl('GET','/api/balances')
        account=curl('POST','/api/accounts',{'name':'工资卡','brand':'招商银行','card':{'type':'debit','number':'1234 5678'}},expected=201)
        ap='/api/accounts/'+str(account['id'])
        curl('PUT',ap,{'name':'工资卡','brand':'招商银行','card':{'type':'credit','number':'12345678'}})
        curl('POST',ap+'/move',{'to':0})
        adjust=curl('POST',ap+'/adjust',{'balance_cents':123456,'occurred_at':'2026-11-17T04:30:00Z'})
        curl('POST',ap+'/adjust',{'balance_cents':123456})
        curl('DELETE',ap,expected=409)
        curl('PUT','/api/settings',{'cycle_day':25,'budget_cents':300000})
        menu=curl('POST','/recognize',extra=['--form','image=@src/api/test.png'])
        # 快捷指令响应不在 /api 下，直接解析。
        menu=json.loads(menu)
        draft=json.loads(base64.urlsafe_b64decode(menu['draft']+'='*(-len(menu['draft'])%4)))
        rid=draft['id']
        curl('POST','/step',{'draft':menu['draft'],'field':menu['field'],'value':'✅ 确认记账'})
        curl('GET','/api/recognitions/'+rid)
        for _ in range(50):
            with sqlite3.connect(tmp/'app.db') as conn: image=conn.execute('SELECT image_path FROM recognitions WHERE draft_id=?',(rid,)).fetchone()[0]
            if pathlib.Path(image).exists():break
            time.sleep(.02)
        curl('GET','/api/recognitions/'+rid+'/image',binary=True)
        csv=curl('GET','/api/export.csv'); assert csv.startswith(b'\xef\xbb\xbf')
        curl('DELETE','/api/entries/'+expense['id'],expected=204)
        curl('DELETE','/api/entries/'+expense['id'],expected=404)
        curl('DELETE','/api/entries/'+adjust['entry']['id'],expected=204)
        listed=curl('GET',q)
        screenshot=next(e for e in listed['entries'] if e['recognition_id']==rid)
        curl('DELETE','/api/entries/'+screenshot['id'],expected=204)
        curl('DELETE',ap,expected=204)
        curl('PUT','/api/password',{'current':'smoke-password','new':'changed-password'},expected=204,extra=['--cookie-jar',str(tmp/'cookie')])
        curl('GET','/api/settings')
        curl('POST','/api/login',{'username':'dzmfg','password':'smoke-password'},expected=401,auth=False)
        curl('POST','/api/login',{'username':'dzmfg','password':'changed-password'},expected=204,auth=False,extra=['--cookie-jar',str(tmp/'cookie')])
        curl('POST','/api/logout',expected=204)
        report.append('PASS：所有 Web 路由及 /recognize、/step 已由真实 curl 请求验证。')
    finally:
        proc.terminate();proc.wait(timeout=10);log.close()
(root/'docs/blueprints/web-api-curl.txt').write_text('本地接口验收样例，2026-11-17；临时数据库 + Mock 识别。\n使用示例 token，无真实用户数据；端口、临时文件路径和 UUID 为本次运行值。\n\n'+'\n'.join(report))
print('PASS: migration + all routes; docs/blueprints/web-api-curl.txt')
