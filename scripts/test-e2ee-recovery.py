#!/usr/bin/env python3
"""Isolated API + real native Tauri IPC process-restart acceptance probe."""
import tempfile
import http.client as http_client
import urllib.parse,base64,concurrent.futures,json,os,secrets,subprocess,time,urllib.request,urllib.error
from pathlib import Path
Q=Path(os.environ.get("OPENCORDE_QA_OUTPUT") or tempfile.mkdtemp(prefix="opencorde-e2ee-qa-"))
Q.mkdir(parents=True,exist_ok=True,mode=0o700)
BASE=os.environ['OPENCORDE_QA_API']
assert BASE.startswith('http://127.0.0.1:'),'isolated loopback only'
BINARY=os.environ['OPENCORDE_QA_IPC']
assert os.environ.get('OPENCORDE_TEST_OS_KEYS')=='1','run inside an isolated real OS-keyring session'
ROOT=Q/('transport-'+str(int(time.time())));ROOT.mkdir(mode=0o700)
events=[];accounts=[];results=[]
def http(method,path,body=None,user=0,allowed=(200,)):
    headers={'Content-Type':'application/json'}
    if accounts and user is not None:headers['Authorization']='Bearer '+accounts[user]['token']
    address=urllib.parse.urlparse(BASE)
    client_index=len(accounts) if user is None else user
    connection=http_client.HTTPConnection(address.hostname,address.port,timeout=30,source_address=(f'127.0.0.{client_index+2}',0))
    connection.request(method,address.path+path,body=None if body is None else json.dumps(body).encode(),headers=headers)
    response=connection.getresponse();code=response.status;data=response.read();connection.close()
    events.append({'method':method,'path':path,'status':code})
    assert code in allowed,(method,path,code,data[:160])
    return json.loads(data) if data else None
def native(i,command,args=None,allow_error=False):
    folder=ROOT/str(i);folder.mkdir(exist_ok=True,mode=0o700)
    request={'server_url':BASE.removesuffix('/api/v1'),'user':accounts[i]['id'],'command':command,'args':args or {}}
    (folder/'request.json').write_text(json.dumps(request));(folder/'request.json').chmod(0o600)
    env=os.environ.copy();env['OPENCORDE_IPC_TEST_DIRECTORY']=str(folder)
    with (folder/'worker.log').open('w') as log:
        run=subprocess.run([BINARY,'--exact','ipc_process_worker','--nocapture'],env=env,stdout=log,stderr=subprocess.STDOUT)
    assert run.returncode==0,('worker',command,run.returncode)
    value=json.loads((folder/'response.json').read_text())
    if allow_error:return value
    assert 'error' not in value,(command,value)
    return value['result']
def status(i):return native(i,'crypto_status')
def channel_state(i,cid):
    return next((c for c in status(i)['channels'] if c['channel_id']==cid),None)
def publish(i):
    value=native(i,'crypto_key_package',{'confirmed_hex':None})
    package=value['key_package_hex']
    wire=base64.urlsafe_b64encode(bytes.fromhex(package)).decode().rstrip('=')
    http('POST','/users/me/key-packages',{'key_package':wire},i,(201,))
    # Deliberately retry upload after native process restart.
    assert status(i)['key_package_hex']==package
    http('POST','/users/me/key-packages',{'key_package':wire},i,(201,))
    native(i,'crypto_key_package',{'confirmed_hex':package})
def catchup(i,cid):
    current=channel_state(i,cid)
    while True:
        page=http('GET',f'/channels/{cid}/e2ee/v2/events?after={current["sequence"]}',user=i)
        if page['events']:native(i,'crypto_apply_events',{'channel_id':cid,'events':page['events']})
        if not page['has_more']:break
        current=channel_state(i,cid)
def submit(i,prepared):
    cid=prepared['channel_id']
    event=http('POST',f'/channels/{cid}/e2ee/v2/events',prepared['operation'],i)
    # Lose HTTP response, restart, retry the exact durable pending operation.
    assert status(i)['pending']==prepared
    again=http('POST',f'/channels/{cid}/e2ee/v2/events',status(i)['pending']['operation'],i)
    assert again==event
    native(i,'crypto_accept_event',{'event':again})
    return event
def join(i,cid):
    w=http('GET',f'/channels/{cid}/e2ee/v2/welcome',user=i)
    assert http('GET',f'/channels/{cid}/e2ee/v2/welcome',user=i)==w
    native(i,'crypto_join',{'channel_id':cid,'welcome':w})
    # Crash after durable join but before relay acknowledgment.
    native(i,'crypto_join',{'channel_id':cid,'welcome':w})
    http('POST',f'/channels/{cid}/e2ee/v2/welcome/ack',{'operation_id':w['operation_id']},i)
    http('POST',f'/channels/{cid}/e2ee/v2/welcome/ack',{'operation_id':w['operation_id']},i)
    native(i,'crypto_ack_welcome',{'channel_id':cid,'operation_id':w['operation_id']})
def add(cid,indices):
    targets=[]
    for i in indices:
        publish(i)
        package=http('GET',f'/users/{accounts[i]["id"]}/key-packages/one?channel_id={cid}')
        raw=base64.urlsafe_b64decode(package['key_package']+'='*(-len(package['key_package'])%4))
        targets.append({'user_id':accounts[i]['id'],'key_package_hex':raw.hex()})
    p=native(0,'crypto_prepare_members',{'channel_id':cid,'added':targets,'removed':None})
    return submit(0,p)
def message(i,cid,text):
    catchup(i,cid)
    p=native(i,'crypto_prepare_message',{'channel_id':cid,'plaintext':text,'reply_to_id':None,'attachments':[]})
    return submit(i,p)
def verify(i,cid,event,text):
    catchup(i,cid)
    listing=http('GET',f'/channels/{cid}/messages?limit=100',user=i)
    projection=next(m for m in listing if m['id']==event['message_id'])
    assert projection['content'].startswith('enc:') and text not in json.dumps(listing)
    got=native(i,'crypto_cached_message',{'channel_id':cid,'message_id':event['message_id'],'ciphertext_hex':projection['content'][4:]})
    assert got==text,(i,got)
def group(size,sid):
    ch=http('POST',f'/servers/{sid}/channels',{'name':f'encrypted-{size}','channel_type':0},allowed=(200,201))
    cid=ch['id'];http('PATCH',f'/channels/{cid}',{'e2ee_enabled':True})
    submit(0,native(0,'crypto_prepare_init',{'channel_id':cid,'allow_legacy_rejoin':False}))
    add(cid,list(range(1,size)))
    for i in range(1,size):join(i,cid)
    # All participants use independently restarted processes.
    last=None
    for i in range(size):last=message(i,cid,f'private-{size}-{i}')
    for i in range(size):verify(i,cid,last,f'private-{size}-{size-1}')
    # Competing application operations share one cursor; one must rebase.
    for i in (0,1):catchup(i,cid)
    prepared=[native(i,'crypto_prepare_message',{'channel_id':cid,'plaintext':f'concurrent-{i}','reply_to_id':None,'attachments':[]}) for i in (0,1)]
    def race(i):
        try:return i,http('POST',f'/channels/{cid}/e2ee/v2/events',prepared[i]['operation'],i)
        except AssertionError as error:
            assert error.args[0][2]==409;return i,None
    with concurrent.futures.ThreadPoolExecutor(max_workers=2) as executor:out=list(executor.map(race,(0,1)))
    winner=next(i for i,e in out if e);loser=1-winner
    native(winner,'crypto_accept_event',{'event':dict(out)[winner]})
    native(loser,'crypto_reject_conflict',{'operation_id':prepared[loser]['operation']['operation_id']})
    catchup(loser,cid)
    retry=native(loser,'crypto_retry_operation')
    submit(loser,retry)
    # Last peer stays offline across removal and re-add of another peer.
    for i in (0,1,2):catchup(i,cid)
    removed=native(0,'crypto_prepare_members',{'channel_id':cid,'added':[],'removed':accounts[1]['id']})
    app=native(2,'crypto_prepare_message',{'channel_id':cid,'plaintext':'wins against staged commit','reply_to_id':None,'attachments':[]})
    submit(2,app)
    http('POST',f'/channels/{cid}/e2ee/v2/events',removed['operation'],0,(409,))
    native(0,'crypto_reject_conflict',{'operation_id':removed['operation']['operation_id']})
    catchup(0,cid)
    removed_retry=native(0,'crypto_retry_operation')
    assert removed_retry['operation']['operation_id']!=removed['operation']['operation_id']
    submit(0,removed_retry)
    http('GET',f'/channels/{cid}/e2ee/v2/events?after=0',user=1,allowed=(403,))
    http('GET',f'/channels/{cid}/messages',user=1,allowed=(403,))
    http('POST',f'/channels/{cid}/e2ee/v2/events',prepared[1]['operation'],1,(403,))
    fresh=message(0,cid,'after-removal')
    # Explicit removed-device rejoin; old state is never silently overwritten.
    native(1,'crypto_request_rejoin',{'channel_id':cid})
    add(cid,[1]);join(1,cid)
    newest=message(0,cid,'after-rejoin')
    verify(size-1,cid,newest,'after-rejoin')
    verify(1,cid,newest,'after-rejoin')
    assert all(m['id']!=fresh['message_id'] for m in http('GET',f'/channels/{cid}/messages',user=1))
    results.append({'members':size,'process_restart_per_operation':True,'ciphertext_projection':True,'retry_dedup':True,'concurrent_rebase':True,'staged_commit_rebase':True,'offline_catchup':True,'removed_access_denied':True,'explicit_rejoin':True,'channel_id':cid})
    return cid
try:
    for i in range(20):
        username=f'recoveryqa_{int(time.time())}_{i}'
        account={'username':username,'email':username+'@example.invalid','password':secrets.token_urlsafe(24)}
        registered=http('POST','/auth/register',account,user=None,allowed=(200,201))
        account['id']=registered['user']['id']
        login=http('POST','/auth/login',{'email':account['email'],'password':account['password']},user=None)
        account['token']=login['access_token'];accounts.append(account)
    server=http('POST','/servers',{'name':'Recovery isolated QA'},allowed=(200,201));sid=server['id']
    private={'accounts':[{k:v for k,v in a.items() if k!='token'} for a in accounts],'server_id':sid,'api_base':BASE}
    (Q/'transport-fixture.json').write_text(json.dumps(private));(Q/'transport-fixture.json').chmod(0o600)
    invite=http('POST',f'/servers/{sid}/invites',{'max_uses':25,'expires_in':3600},allowed=(200,201))
    for i in range(1,20):http('POST',f'/invites/{invite["code"]}/join',{},i,(200,201,204))
    # Stress first; removed peer is quarantined only at the end of each group.
    group(5,sid)
    cid=group(20,sid)
    fixture={'accounts':[{k:v for k,v in a.items() if k!='token'} for a in accounts],'server_id':sid,'channel_id':cid,'api_base':BASE}
    (Q/'transport-fixture.json').write_text(json.dumps(fixture));(Q/'transport-fixture.json').chmod(0o600)
    (Q/'transport-evidence.json').write_text(json.dumps({'results':results,'http':events},indent=2))
    print(json.dumps({'results':results,'fixture':str(Q/'transport-fixture.json')}))
finally:
    (Q/'transport-http.json').write_text(json.dumps(events))
