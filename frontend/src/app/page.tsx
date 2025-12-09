"use client";
import { useState, useRef, useEffect } from 'react';
import { Send, Upload, ShieldAlert, Check, X, FileText, Loader2 } from 'lucide-react';
import { UserButton, useUser } from "@clerk/nextjs"; // 1. Import Clerk Hooks

export default function Home() {
  const API_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8080/api/v1';

  // 2. Get Real User Data
  const { user, isLoaded } = useUser();

  const [activeTab, setActiveTab] = useState<'chat' | 'upload' | 'approvals'>('chat');
  
  // --- CHAT STATE ---
  const [query, setQuery] = useState('');
  const [chatHistory, setChatHistory] = useState<any[]>([]);
  const [loading, setLoading] = useState(false);

  // --- UPLOAD STATE ---
  const [uploadStatus, setUploadStatus] = useState('');
  const fileInputRef = useRef<HTMLInputElement>(null);

  // --- APPROVALS STATE ---
  const [pendingActions, setPendingActions] = useState<any[]>([]);

  // Fetch actions ONLY when user is loaded
  useEffect(() => {
    if (isLoaded && user) {
      fetchPendingActions();
    }
  }, [activeTab, isLoaded, user]);

  const fetchPendingActions = async () => {
    if (!user) return;
    try {
      // 3. Send Real User ID in params
      const res = await fetch(`${API_URL}/pending?user_id=${user.id}`);
      if (res.ok) {
        const data = await res.json();
        const actions = Array.isArray(data) ? data : data.actions || [];
        setPendingActions(actions);
      }
    } catch (err) {
      console.error("Failed to fetch pending actions", err);
    }
  };

  // 1. HANDLE CHAT
  const handleSearch = async () => {
    if (!query || !user) return;
    setLoading(true);
    
    const newHistory = [...chatHistory, { role: 'user', content: query }];
    setChatHistory(newHistory);
    
    try {
      const res = await fetch(`${API_URL}/query`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ 
            user_id: user.id, // 4. Send Real User ID
            query: query 
        })
      });
      
      if (res.ok) {
        const data = await res.json();
        setChatHistory([...newHistory, { 
          role: 'ai', 
          content: data.summary, 
          citations: data.citations 
        }]);
        // Refresh approvals if AI said it created one
        fetchPendingActions();
      } else {
        throw new Error('Backend error');
      }
    } catch (err) {
      setChatHistory([...newHistory, { role: 'ai', content: "Error: Could not connect to Orchestrator." }]);
    }
    setLoading(false);
    setQuery('');
  };

  // 2. HANDLE UPLOAD
  const handleUpload = async (e: React.ChangeEvent<HTMLInputElement>) => {
    if (!e.target.files?.[0] || !user) return;
    const file = e.target.files[0];
    const formData = new FormData();
    formData.append('file', file);
    // Note: We send user_id, though currently backend might rely on its own logic.
    // Good for future proofing RBAC.
    formData.append('user_id', user.id); 

    setUploadStatus('Uploading...');

    try {
      const res = await fetch(`${API_URL}/ingest`, {
        method: 'POST',
        body: formData,
      });
      
      if (res.ok) {
        const data = await res.json();
        setUploadStatus(`Success! Document ID: ${data.document_id}`);
      } else {
        setUploadStatus('Upload failed (Server Error).');
      }
    } catch (err) {
      setUploadStatus('Upload failed (Network Error).');
    }
  };

  // 3. HANDLE APPROVALS
  const handleApprove = async (id: string, approved: boolean) => {
    if (!user) return;
    try {
      const res = await fetch(`${API_URL}/approve`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          action_id: id,
          approved: approved,
          // 5. Send Real User Signature (Name or Email)
          user_signature: user.fullName || user.primaryEmailAddress?.emailAddress || user.id
        })
      });

      if (res.ok) {
        setPendingActions(pendingActions.filter(a => a.id !== id));
      } else {
        alert("Error sending approval to backend.");
      }
    } catch (err) {
      console.error("Approval failed", err);
      alert("Network error.");
    }
  };

  // 6. Show Loading State while Clerk connects
  if (!isLoaded) {
    return (
        <div className="flex min-h-screen bg-slate-950 items-center justify-center text-slate-400">
            <Loader2 className="animate-spin mr-2" /> Loading User Data...
        </div>
    );
  }

  // 7. If not signed in, Clerk middleware will redirect, but we render nothing just in case
  if (!user) return null;

  return (
    <main className="flex min-h-screen bg-slate-950 text-slate-100 font-sans">
      
      {/* SIDEBAR */}
      <div className="w-64 border-r border-slate-800 p-6 flex flex-col gap-6">
        
        {/* USER PROFILE SECTION */}
        <div className="flex items-center gap-3 pb-4 border-b border-slate-800">
            <UserButton afterSignOutUrl="/"/>
            <div className="overflow-hidden">
                <h1 className="text-sm font-bold bg-gradient-to-r from-blue-400 to-purple-500 bg-clip-text text-transparent truncate">
                Agentic AI
                </h1>
                <p className="text-xs text-slate-500 truncate" title={user.primaryEmailAddress?.emailAddress}>
                    {user.fullName || 'User'}
                </p>
            </div>
        </div>

        <nav className="flex flex-col gap-2">
          <button onClick={() => setActiveTab('chat')} className={`flex items-center gap-3 p-3 rounded-lg text-sm font-medium transition ${activeTab === 'chat' ? 'bg-blue-600 text-white' : 'hover:bg-slate-900 text-slate-400'}`}>
            <Send size={18} /> Knowledge Chat
          </button>
          <button onClick={() => setActiveTab('upload')} className={`flex items-center gap-3 p-3 rounded-lg text-sm font-medium transition ${activeTab === 'upload' ? 'bg-blue-600 text-white' : 'hover:bg-slate-900 text-slate-400'}`}>
            <Upload size={18} /> Ingest Data
          </button>
          <button onClick={() => setActiveTab('approvals')} className={`flex items-center gap-3 p-3 rounded-lg text-sm font-medium transition ${activeTab === 'approvals' ? 'bg-blue-600 text-white' : 'hover:bg-slate-900 text-slate-400'}`}>
            <ShieldAlert size={18} /> Pending Actions
            {pendingActions.length > 0 && <span className="ml-auto bg-red-500 text-white text-xs px-2 py-0.5 rounded-full">{pendingActions.length}</span>}
          </button>
        </nav>
      </div>

      {/* MAIN CONTENT */}
      <div className="flex-1 p-8 overflow-y-auto">
        
        {/* VIEW: CHAT */}
        {activeTab === 'chat' && (
          <div className="max-w-4xl mx-auto flex flex-col h-[90vh]">
            <div className="flex-1 overflow-y-auto mb-6 space-y-6 pr-4">
              {chatHistory.length === 0 && (
                <div className="text-center text-slate-500 mt-20">
                  <p className="text-lg">Welcome back, {user.firstName}. <br/>Ask me anything about your documents.</p>
                </div>
              )}
              {chatHistory.map((msg, i) => (
                <div key={i} className={`flex ${msg.role === 'user' ? 'justify-end' : 'justify-start'}`}>
                  <div className={`max-w-[80%] p-4 rounded-xl ${msg.role === 'user' ? 'bg-blue-600' : 'bg-slate-800 border border-slate-700'}`}>
                    <p className="leading-relaxed">{msg.content}</p>
                    {/* Citations Block */}
                    {msg.citations && msg.citations.length > 0 && (
                      <div className="mt-4 pt-4 border-t border-slate-700 text-sm">
                        <p className="text-slate-400 font-semibold mb-2 flex items-center gap-2"><FileText size={14}/> Sources:</p>
                        {msg.citations.map((cite: any, idx: number) => (
                          <div key={idx} className="bg-slate-900 p-2 rounded mb-1 text-xs text-slate-300">
                             Page {cite.page}: "...{cite.text ? cite.text.substring(0, 50) : ''}..."
                          </div>
                        ))}
                      </div>
                    )}
                  </div>
                </div>
              ))}
              {loading && <div className="text-slate-500 flex gap-2"><Loader2 className="animate-spin"/> Thinking...</div>}
            </div>
            
            <div className="flex gap-4">
              <input 
                type="text" 
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                onKeyDown={(e) => e.key === 'Enter' && handleSearch()}
                placeholder="Ex: Please create a ticket for the login failure..."
                className="flex-1 bg-slate-900 border border-slate-700 rounded-lg px-4 py-3 focus:outline-none focus:border-blue-500 transition"
              />
              <button onClick={handleSearch} className="bg-blue-600 hover:bg-blue-500 px-6 rounded-lg font-medium transition">Send</button>
            </div>
          </div>
        )}

        {/* VIEW: UPLOAD */}
        {activeTab === 'upload' && (
          <div className="max-w-2xl mx-auto mt-20 p-10 bg-slate-900 rounded-2xl border border-slate-800 text-center">
            <div className="w-20 h-20 bg-slate-800 rounded-full flex items-center justify-center mx-auto mb-6">
              <Upload size={32} className="text-blue-400"/>
            </div>
            <h2 className="text-2xl font-bold mb-2">Ingest Knowledge</h2>
            <p className="text-slate-400 mb-8">Upload PDF, DOCX, Images, or Audio files.</p>
            
            <input 
              type="file" 
              ref={fileInputRef}
              onChange={handleUpload}
              className="hidden" 
            />
            <button 
              onClick={() => fileInputRef.current?.click()}
              className="bg-blue-600 hover:bg-blue-500 text-white px-8 py-3 rounded-lg font-medium transition w-full"
            >
              Select File
            </button>
            
            {uploadStatus && (
              <div className="mt-6 p-4 bg-slate-950 rounded border border-slate-800 text-sm text-green-400">
                {uploadStatus}
              </div>
            )}
          </div>
        )}

        {/* VIEW: APPROVALS */}
        {activeTab === 'approvals' && (
          <div className="max-w-4xl mx-auto">
            <h2 className="text-2xl font-bold mb-8">Pending Agent Actions</h2>
            <div className="space-y-4">
              {pendingActions.length === 0 ? (
                <div className="text-center p-10 border border-dashed border-slate-800 rounded-xl text-slate-500">
                  No actions pending approval.
                </div>
              ) : ( 
               pendingActions.map((action) => (
                <div key={action.id} className="bg-slate-900 border border-slate-800 p-6 rounded-xl flex items-center justify-between">
                  <div>
                    <div className="flex items-center gap-3 mb-2">
                      <span className="bg-purple-500/20 text-purple-300 text-xs px-2 py-1 rounded uppercase font-bold tracking-wider">
                        {action.action_type || action.type || 'ACTION'}
                      </span>
                      {(action.payload?.confidence || action.confidence) && (
                        <span className="text-slate-500 text-sm">
                          Confidence: {((action.payload?.confidence || action.confidence) * 100).toFixed(0)}%
                        </span>
                      )}
                    </div>
                    <p className="text-lg font-medium">
                        {action.payload?.description || action.description || 'Action requiring approval'}
                    </p>
                  </div>
                  <div className="flex gap-3">
                    <button onClick={() => handleApprove(action.id, false)} className="p-2 hover:bg-red-500/20 text-red-400 rounded-lg transition border border-transparent hover:border-red-500/50">
                      <X size={20}/>
                    </button>
                    <button onClick={() => handleApprove(action.id, true)} className="p-2 bg-green-600 hover:bg-green-500 text-white rounded-lg transition shadow-lg shadow-green-900/20">
                      <Check size={20}/>
                    </button>
                  </div>
                </div>
              )))}
            </div>
          </div>
        )}

      </div>
    </main>
  );
}