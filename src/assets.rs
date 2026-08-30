pub(crate) const PAGE: &str = r##"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width,initial-scale=1">
  <meta name="color-scheme" content="light">
  <meta name="description" content="Write, revise, and publish Knowledge Base articles.">
  <title>Lenso Knowledge Author</title>
  <link rel="stylesheet" href="/knowledge/assets/app.css">
</head>
<body>
  <div class="shell">
    <aside>
      <div class="brand"><span aria-hidden="true">L</span><div><b>Knowledge</b><small>Author workspace</small></div></div>
      <section class="connection" aria-labelledby="connection-title">
        <h2 id="connection-title">Workspace</h2>
        <label>Organization<input id="organization" autocomplete="organization" placeholder="org_demo"></label>
        <label>Bearer token<input id="token" type="password" autocomplete="current-password" placeholder="••••••••"></label>
        <button id="connect" type="button">Open workspace</button>
        <p id="connection-state" role="status">Not connected</p>
      </section>
      <div class="list-heading"><div><p class="eyebrow">ARTICLES</p><h2>Draft library</h2></div><button id="new-draft" class="icon-button" type="button" aria-label="Create a new draft">+</button></div>
      <div id="articles" class="article-list"><p class="empty">Connect to load your drafts.</p></div>
      <button id="load-more" class="quiet hidden" type="button">Load more</button>
    </aside>
    <main>
      <header>
        <div><p class="eyebrow">KNOWLEDGE BASE</p><h1 id="workspace-title">Choose a draft</h1><p id="workspace-meta">Open an existing article or start a focused new draft.</p></div>
        <div class="header-actions"><span id="save-state" role="status"></span><button id="save" class="secondary" type="button" disabled>Save draft</button><button id="publish" type="button" disabled>Publish</button></div>
      </header>
      <section id="welcome" class="welcome">
        <div class="welcome-mark" aria-hidden="true">✦</div>
        <p class="eyebrow">AUTHORING, NOT ADMINISTRATION</p>
        <h2>Turn one solved problem into a reusable answer.</h2>
        <p>Create a draft, refine the Markdown, then publish the exact revision you reviewed. Existing published answers remain available while you edit.</p>
        <button id="welcome-new" type="button">Write an article</button>
      </section>
      <form id="editor" class="editor hidden">
        <div class="field-grid">
          <label>Title<input id="title" name="title" required maxlength="240" placeholder="A clear answer to one problem"></label>
          <label>Slug<input id="slug" name="slug" required maxlength="160" placeholder="clear-answer" pattern="[a-z0-9]+(?:-[a-z0-9]+)*"></label>
        </div>
        <label class="body-label"><span>Article body <small>Markdown</small></span><textarea id="body" name="body_markdown" required maxlength="100000" spellcheck="true" placeholder="# Start with the outcome&#10;&#10;Explain the steps and expected result."></textarea></label>
        <p id="editor-error" class="error" role="alert"></p>
      </form>
    </main>
  </div>
  <div id="toast" role="status" aria-live="polite"></div>
  <script src="/knowledge/assets/app.js" defer></script>
</body>
</html>"##;

pub(crate) const CSS: &str = r#":root{font-family:Inter,ui-sans-serif,-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;color:#20241f;background:#f3f4ef;--ink:#20241f;--muted:#71786f;--line:#d9ddd5;--paper:#fff;--accent:#2f7258;--accent-soft:#e5f0e9;--danger:#ad3d37}*{box-sizing:border-box}body{margin:0;background:linear-gradient(130deg,#f7f8f3 0,#eef1eb 100%);min-width:920px}.shell{display:grid;grid-template-columns:330px 1fr;min-height:100vh}aside{height:100vh;position:sticky;top:0;display:flex;flex-direction:column;padding:24px 18px;border-right:1px solid var(--line);background:#f8f9f5}.brand{display:flex;align-items:center;gap:11px;padding:0 5px 24px}.brand>span{display:grid;place-items:center;width:34px;height:34px;border-radius:11px;color:white;background:linear-gradient(145deg,#47856b,#255e49);box-shadow:0 8px 24px #2f725833}.brand div{display:grid}.brand small{color:var(--muted);font-size:11px;margin-top:2px}.connection{display:grid;grid-template-columns:1fr 1fr;gap:9px;padding:15px;border:1px solid var(--line);border-radius:13px;background:var(--paper);box-shadow:0 4px 18px #25302708}.connection h2{grid-column:1/-1;font-size:13px;margin:0}.connection label{display:grid;gap:5px;font-size:10px;color:var(--muted);text-transform:uppercase;letter-spacing:.06em}.connection button,.connection p{grid-column:1/-1}.connection p{margin:0;color:var(--muted);font-size:11px}.list-heading{display:flex;align-items:end;justify-content:space-between;padding:27px 5px 11px}.list-heading h2{margin:3px 0 0;font-size:17px}.eyebrow{margin:0;color:var(--accent);font-size:9px;font-weight:800;letter-spacing:.15em}.article-list{overflow:auto;display:grid;gap:6px;padding-right:2px}.article-item{width:100%;display:block;padding:13px;text-align:left;border:1px solid transparent;border-radius:10px;background:transparent;color:var(--ink)}.article-item:hover,.article-item.active{border-color:#cbd4cc;background:var(--paper);box-shadow:0 3px 14px #25302708}.article-item strong,.article-item span{display:block;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.article-item strong{font-size:13px}.article-item span{margin-top:5px;color:var(--muted);font-size:10px}.article-item i{display:inline-block;margin-top:8px;padding:3px 6px;border-radius:999px;background:#ecefe9;color:#626960;font-size:9px;font-style:normal;text-transform:uppercase}.article-item i.published{background:var(--accent-soft);color:var(--accent)}.empty{padding:20px 10px;color:var(--muted);font-size:12px;line-height:1.55}.hidden{display:none!important}button,input,textarea{font:inherit}button{padding:9px 13px;border:0;border-radius:8px;background:var(--accent);color:#fff;font-weight:700;cursor:pointer}button:hover:not(:disabled){filter:brightness(1.08)}button:disabled{opacity:.42;cursor:not-allowed}.secondary{border:1px solid var(--line);background:var(--paper);color:var(--ink)}.quiet{margin-top:10px;border:1px solid var(--line);background:transparent;color:var(--muted)}.icon-button{width:31px;height:31px;padding:0;font-size:20px}.connection input,input,textarea{width:100%;padding:9px 10px;border:1px solid var(--line);border-radius:8px;background:#fbfcf9;color:var(--ink);outline:none}.connection input{font-size:11px}.connection input:focus,input:focus,textarea:focus{border-color:#6e9d87;box-shadow:0 0 0 3px #3e7d6120}main{min-width:0;padding:31px 38px 48px}main>header{display:flex;align-items:center;justify-content:space-between;gap:24px;padding-bottom:23px;border-bottom:1px solid var(--line)}h1{font-size:25px;letter-spacing:-.035em;margin:5px 0}.header-actions{display:flex;align-items:center;gap:9px}.header-actions span{min-width:110px;color:var(--muted);font-size:11px;text-align:right}#workspace-meta{margin:0;color:var(--muted);font-size:12px}.welcome{max-width:650px;margin:12vh auto 0;padding:48px;border:1px solid var(--line);border-radius:18px;background:var(--paper);box-shadow:0 16px 50px #29342c10}.welcome-mark{display:grid;place-items:center;width:42px;height:42px;margin-bottom:26px;border-radius:13px;background:var(--accent-soft);color:var(--accent);font-size:21px}.welcome h2{max-width:520px;margin:9px 0 14px;font-size:31px;letter-spacing:-.035em}.welcome>p:not(.eyebrow){color:var(--muted);line-height:1.7}.welcome button{margin-top:10px}.editor{padding-top:28px}.field-grid{display:grid;grid-template-columns:2fr 1fr;gap:14px}.editor label{display:grid;gap:7px;color:#565d55;font-size:11px;font-weight:700}.editor label small{float:right;color:var(--muted);font-weight:500}.body-label{margin-top:17px}.editor textarea{min-height:calc(100vh - 280px);padding:20px;resize:vertical;font-family:ui-monospace,SFMono-Regular,Menlo,monospace;font-size:13px;line-height:1.7;background:var(--paper)}.error{min-height:20px;margin:10px 0 0;color:var(--danger);font-size:12px}#toast{position:fixed;right:24px;bottom:24px;opacity:0;transform:translateY(12px);padding:11px 15px;border-radius:9px;background:var(--ink);color:white;font-size:12px;transition:.18s;pointer-events:none}#toast.show{opacity:1;transform:none}@media(max-width:1050px){.shell{grid-template-columns:285px 1fr}main{padding:26px}.header-actions span{display:none}}"#;

pub(crate) const JS: &str = r"const $=selector=>document.querySelector(selector);
const state={organization:'',token:'',articles:[],nextCursor:null,current:null,dirty:false};
function requestId(){return crypto.randomUUID()}
function credentials(){state.organization=$('#organization').value.trim();state.token=$('#token').value.trim();if(!state.organization||!state.token)throw new Error('Enter an organization and Bearer token.');localStorage.setItem('lenso.knowledge.organization',state.organization)}
async function api(path,options={}){const headers={...(options.headers||{}),Authorization:`Bearer ${state.token}`};if(options.body)headers['Content-Type']='application/json';const response=await fetch(path,{...options,headers});if(!response.ok){let problem={};try{problem=await response.json()}catch{}const error=new Error(problem.detail||`Request failed (${response.status})`);error.status=response.status;error.code=problem.type||problem.title;throw error}return response.json()}
function toast(message){const node=$('#toast');node.textContent=message;node.classList.add('show');setTimeout(()=>node.classList.remove('show'),2200)}
function busy(value){$('#connect').disabled=value;$('#new-draft').disabled=value;$('#save').disabled=value||!state.current;$('#publish').disabled=value||!state.current?.article_id}
function date(value){try{return new Intl.DateTimeFormat(undefined,{dateStyle:'medium',timeStyle:'short'}).format(new Date(value))}catch{return value}}
function setDirty(value){state.dirty=value;$('#save-state').textContent=value?'Unsaved changes':''}

async function loadArticles(reset=true){credentials();busy(true);$('#connection-state').textContent='Loading drafts…';try{const params=new URLSearchParams({organization_id:state.organization,limit:'40'});if(!reset&&state.nextCursor)params.set('cursor',state.nextCursor);const result=await api(`/api/knowledge/articles?${params}`);state.articles=reset?result.articles:[...state.articles,...result.articles];state.nextCursor=result.next_cursor;renderArticles();$('#connection-state').textContent=`Connected · ${state.articles.length} loaded`;$('#load-more').classList.toggle('hidden',!state.nextCursor)}catch(error){$('#connection-state').textContent=error.message}finally{busy(false)}}
function renderArticles(){const host=$('#articles');host.replaceChildren();if(!state.articles.length){const empty=document.createElement('p');empty.className='empty';empty.textContent='No articles yet. Create the first draft for this organization.';host.append(empty);return}state.articles.forEach(article=>{const button=document.createElement('button');button.type='button';button.className=`article-item${state.current?.article_id===article.article_id?' active':''}`;const title=document.createElement('strong');title.textContent=article.title;const slug=document.createElement('span');slug.textContent=article.slug;const status=document.createElement('i');status.className=article.latest_publication_revision?'published':'';status.textContent=article.latest_publication_revision?'Published':'Draft';button.append(title,slug,status);button.addEventListener('click',()=>openDraft(article.article_id));host.append(button)})}
async function openDraft(articleId){if(state.dirty&&!confirm('Discard unsaved changes and open another draft?'))return;busy(true);$('#editor-error').textContent='';try{const params=new URLSearchParams({organization_id:state.organization});const draft=await api(`/api/knowledge/articles/${encodeURIComponent(articleId)}?${params}`);showDraft(draft);renderArticles()}catch(error){$('#editor-error').textContent=error.message}finally{busy(false)}}
function showDraft(draft){state.current=draft;$('#welcome').classList.add('hidden');$('#editor').classList.remove('hidden');$('#title').value=draft.title||'';$('#slug').value=draft.slug||'';$('#slug').disabled=Boolean(draft.article_id);$('#body').value=draft.body_markdown||'';$('#workspace-title').textContent=draft.article_id?draft.title:'New article';$('#workspace-meta').textContent=draft.article_id?`Draft ${draft.revision} · updated ${date(draft.updated_at)}`:'Choose a permanent slug, then save the first draft.';$('#save').disabled=false;$('#publish').disabled=!draft.article_id;setDirty(false)}
function newDraft(){if(state.dirty&&!confirm('Discard unsaved changes and start a new draft?'))return;showDraft({article_id:null,title:'',slug:'',body_markdown:'',revision:null});$('#title').focus()}
async function saveDraft(){if(!$('#editor').reportValidity())return;busy(true);$('#editor-error').textContent='';try{const common={organization_id:state.organization,title:$('#title').value.trim(),body_markdown:$('#body').value,idempotency_key:requestId()};let saved;if(state.current.article_id){saved=await api(`/api/knowledge/articles/${encodeURIComponent(state.current.article_id)}`,{method:'PATCH',body:JSON.stringify({...common,article_id:state.current.article_id,expected_revision:state.current.revision})})}else{saved=await api('/api/knowledge/articles',{method:'POST',body:JSON.stringify({...common,slug:$('#slug').value.trim()})})}showDraft(saved);toast('Draft saved');await loadArticles(true)}catch(error){$('#editor-error').textContent=error.status===409?'This draft changed or the slug is already used. Reload before trying again.':error.message}finally{busy(false)}}
async function publish(){if(!state.current?.article_id)return;if(!confirm('Publish this exact saved revision?'))return;busy(true);$('#editor-error').textContent='';try{await api(`/api/knowledge/articles/${encodeURIComponent(state.current.article_id)}/publish`,{method:'POST',body:JSON.stringify({organization_id:state.organization,article_id:state.current.article_id,expected_revision:state.current.revision,idempotency_key:requestId()})});toast('Article published');await openDraft(state.current.article_id);await loadArticles(true)}catch(error){$('#editor-error').textContent=error.status===409?'The saved revision changed before publication. Reload the draft and review it again.':error.message}finally{busy(false)}}

$('#connect').addEventListener('click',()=>loadArticles(true));
$('#load-more').addEventListener('click',()=>loadArticles(false));
$('#new-draft').addEventListener('click',newDraft);
$('#welcome-new').addEventListener('click',newDraft);
$('#save').addEventListener('click',saveDraft);
$('#publish').addEventListener('click',publish);
$('#editor').addEventListener('input',()=>setDirty(true));
window.addEventListener('beforeunload',event=>{if(state.dirty){event.preventDefault();event.returnValue=''}});
$('#organization').value=localStorage.getItem('lenso.knowledge.organization')||'';
";
