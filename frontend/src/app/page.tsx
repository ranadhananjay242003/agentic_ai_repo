"use client";
import { useState, useRef, useEffect } from 'react';
import { 
  Send, Upload, ShieldAlert, Check, X, 
  FileText, Loader2, Code, Zap, Plus, 
  MessageSquare, LayoutDashboard 
} from 'lucide-react';
import { UserButton, useUser } from "@clerk/nextjs"; 

export default function Home() {
  const API_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8080/api/v1';
  const { user, isLoaded } = useUser();
  
  const [activeTab, setActiveTab] = useState<'chat' | 'upload' | 'approvals'>('chat');
  const [query, setQuery] = useState('');
  const [chatHistory, setChatHistory] = useState<any[]>([]);
  const [loading, setLoading] = useState(false);
  const [uploadStatus, setUploadStatus] = useState('');
  const fileInputRef = useRef<HTMLInputElement>(null);
  const [pendingActions, setPendingActions] = useState<any[]>([]);
  const chatScrollRef = useRef<HTMLDivElement>(null);

  // Auto-scroll to bottom
  useEffect(() => {
    if (chatScrollRef.current) {
        chatScrollRef.current.scrollTop = chatScrollRef.current.scrollHeight;
    }
  }, [chatHistory, loading]);

  useEffect(() => {
    if (isLoaded && user) { fetchPendingActions(); }
  }, [activeTab, isLoaded, user]);

  // --- NEW FEATURE: Clear Chat ---
  const handleNewChat = () => {
    setChatHistory([]);
    setQuery('');
    setActiveTab('chat');
  };

  const fetchPendingActions = async () => {
    if (!user) return;
    try {
      const res = await fetch(`${API_URL}/pending?user_id=${user.id}`);
      if (res.ok) {
        const data = await res.json();
        setPendingActions(Array.isArray(data) ? data : data.actions || []);
      }
    } catch (err) { /* silent fail */ }
  };
  
  const handleSearch = async () => {
    if (!query || !user) return;
    setLoading(true);
    const newHistory = [...chatHistory, { role: 'user', content: query }];
    setChatHistory(newHistory);
    setQuery(''); // Clear input immediately
    
    try {
      const res = await fetch(`${API_URL}/query`, {
        method: 'POST', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ user_id: user.id, query: query })
      });
      if (res.ok) {
        const data = await res.json();
        setChatHistory([...newHistory, { role: 'ai', content: data.summary, citations: data.citations }]);
        fetchPendingActions();
      } else { throw new Error('Backend error'); }
    } catch (err) {
      setChatHistory([...newHistory, { role: 'ai', content: "Error: Could not connect to Orchestrator." }]);
    }
    setLoading(false);
  };

  const handleUpload = async (e: React.ChangeEvent<HTMLInputElement>) => {
    if (!e.target.files?.[0] || !user) return;
    const file = e.target.files[0];
    const formData = new FormData();
    formData.append('file', file);
    formData.append('user_id', user.id); 

    setUploadStatus('Uploading...');

    try {
      const res = await fetch(`${API_URL}/ingest`, { method: 'POST', body: formData });
      if (res.ok) {
        const data = await res.json();
        setUploadStatus(`Success! Document ID: ${data.document_id}`);
      } else { setUploadStatus('Upload failed (Server Error).'); }
    } catch (err) { setUploadStatus('Upload failed (Network Error).'); }
  };
  
  const handleApprove = async (id: string, approved: boolean) => {
    if (!user) return;
    try {
      const res = await fetch(`${API_URL}/approve`, {
        method: 'POST', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ action_id: id, approved: approved, user_signature: user.fullName || user.primaryEmailAddress?.emailAddress || user.id })
      });
      if (res.ok) { setPendingActions(pendingActions.filter(a => a.id !== id)); } 
    } catch (err) { console.error("Approval failed", err); }
  };
  
  // Custom Renderer for Code Execution
  const renderMessageContent = (content: string) => {
    if (content.startsWith('🤖 **I wrote and executed a Python script')) {
        const code_parts = content.split('```');
        const introText = code_parts[0].replace('**', '').replace('**', '').trim();
        const codeBlock = code_parts.length > 1 ? code_parts[1].trim() : null;
        const resultText = code_parts.length > 2 ? code_parts[2].replace('Result:', '').trim() : null;

        return (
            <div className="w-full">
                <p className="leading-relaxed font-semibold mb-3 flex items-center text-purple-400">
                    <Code size={18} className='mr-2'/> {introText}
                </p>
                {codeBlock && (
                    <div className="rounded-md overflow-hidden border border-zinc-700 bg-[#0d1117]">
                        <div className="bg-zinc-800 px-3 py-1 text-xs text-zinc-400 font-mono border-b border-zinc-700">Python</div>
                        <pre className="p-3 text-sm font-mono text-gray-300 overflow-x-auto custom-scrollbar">
                            {codeBlock.replace('python', '').trim()}
                        </pre>
                    </div>
                )}
                {resultText && (
                    <div className="mt-3 flex items-start gap-2">
                        <Zap size={16} className='text-yellow-400 mt-1'/>
                        <div>
                            <p className="text-xs text-zinc-500 font-bold uppercase tracking-wider mb-1">Result</p>
                            <p className="text-md font-mono text-white bg-green-900/30 px-3 py-2 rounded-md border border-green-700/50">
                                {resultText}
                            </p>
                        </div>
                    </div>
                )}
            </div>
        );
    }
    return <p className="leading-relaxed whitespace-pre-wrap">{content}</p>;
  }

  if (!isLoaded) { return (<div className="flex min-h-screen bg-black items-center justify-center text-zinc-500"><Loader2 className="animate-spin mr-2" /> Initializing...</div>); }
  if (!user) return null;

  return (
    <main className="flex h-screen bg-black text-zinc-100 font-sans overflow-hidden">
      
      {/* SIDEBAR */}
      <div className="w-72 bg-zinc-900 border-r border-zinc-800 flex flex-col shadow-xl z-20">
        
        {/* User Header */}
        <div className="p-4 border-b border-zinc-800">
            <div className="flex items-center gap-3">
                <UserButton afterSignOutUrl="/" appearance={{ elements: { userButtonAvatarBox: "w-9 h-9" } }}/>
                <div className="overflow-hidden">
                    <h1 className="text-md font-bold text-white tracking-tight">Nexus</h1>
                    <p className="text-xs text-zinc-500 truncate">{user.fullName}</p>
                </div>
            </div>
        </div>

        {/* New Chat Button */}
        <div className="p-4 pb-2">
            <button 
                onClick={handleNewChat}
                className="flex items-center justify-center gap-2 w-full bg-white text-black hover:bg-zinc-200 transition-colors py-2.5 rounded-lg font-semibold text-sm shadow-sm"
            >
                <Plus size={18} /> New Chat
            </button>
        </div>

        {/* Navigation */}
        <nav className="flex-1 overflow-y-auto px-2 py-4 space-y-1">
            <div className="text-xs font-semibold text-zinc-500 px-3 mb-2 uppercase tracking-wider">Menu</div>
            
            <button onClick={() => setActiveTab('chat')} className={`w-full flex items-center gap-3 px-3 py-2.5 rounded-md text-sm font-medium transition-all ${activeTab === 'chat' ? 'bg-zinc-800 text-white' : 'text-zinc-400 hover:bg-zinc-800/50 hover:text-zinc-200'}`}>
                <MessageSquare size={18} /> Knowledge Chat
            </button>
            <button onClick={() => setActiveTab('upload')} className={`w-full flex items-center gap-3 px-3 py-2.5 rounded-md text-sm font-medium transition-all ${activeTab === 'upload' ? 'bg-zinc-800 text-white' : 'text-zinc-400 hover:bg-zinc-800/50 hover:text-zinc-200'}`}>
                <Upload size={18} /> Ingest Data
            </button>
            <button onClick={() => setActiveTab('approvals')} className={`w-full flex items-center gap-3 px-3 py-2.5 rounded-md text-sm font-medium transition-all ${activeTab === 'approvals' ? 'bg-zinc-800 text-white' : 'text-zinc-400 hover:bg-zinc-800/50 hover:text-zinc-200'}`}>
                <div className="relative">
                    <ShieldAlert size={18} />
                    {pendingActions.length > 0 && <span className="absolute -top-1 -right-1 w-2.5 h-2.5 bg-red-500 rounded-full animate-pulse"></span>}
                </div>
                Pending Actions
                {pendingActions.length > 0 && <span className="ml-auto text-xs bg-red-500/10 text-red-400 px-2 py-0.5 rounded-full border border-red-500/20">{pendingActions.length}</span>}
            </button>
        </nav>

        <div className="p-4 border-t border-zinc-800 text-xs text-zinc-600 text-center">
            v1.0.0 • Production Ready
        </div>
      </div>

      {/* MAIN AREA */}
      <div className="flex-1 flex flex-col relative bg-black">
        
        {/* CHAT VIEW */}
        {activeTab === 'chat' && (
          <>
            <div className="flex-1 overflow-y-auto p-4 md:p-8 scroll-smooth" ref={chatScrollRef}>
              {chatHistory.length === 0 ? (
                <div className="h-full flex flex-col items-center justify-center text-zinc-500 space-y-4">
                  <div className="w-16 h-16 bg-zinc-900 rounded-2xl flex items-center justify-center mb-2 border border-zinc-800">
                    <LayoutDashboard size={32} className="text-zinc-400"/>
                  </div>
                  <h3 className="text-xl font-semibold text-white">How can Nexus help you today?</h3>
                  <div className="grid grid-cols-1 md:grid-cols-2 gap-3 max-w-lg w-full">
                    <button onClick={() => setQuery("Calculate the sum of fibonacci numbers")} className="p-3 bg-zinc-900/50 hover:bg-zinc-900 border border-zinc-800 hover:border-zinc-700 rounded-xl text-sm text-left transition text-zinc-300">
                        🧮 Calculate Fibonacci numbers
                    </button>
                    <button onClick={() => setQuery("Create a Jira ticket for system outage")} className="p-3 bg-zinc-900/50 hover:bg-zinc-900 border border-zinc-800 hover:border-zinc-700 rounded-xl text-sm text-left transition text-zinc-300">
                        🎫 Create a Jira Ticket
                    </button>
                  </div>
                </div>
              ) : (
                <div className="max-w-3xl mx-auto space-y-6 pb-4">
                  {chatHistory.map((msg, i) => (
                    <div key={i} className={`flex ${msg.role === 'user' ? 'justify-end' : 'justify-start'}`}>
                      <div className={`max-w-[85%] rounded-2xl p-5 ${
                          msg.role === 'user' 
                          ? 'bg-white text-black' 
                          : 'bg-zinc-900 border border-zinc-800 text-zinc-100'
                        }`}>
                        {renderMessageContent(msg.content)}
                        
                        {/* Citations */}
                        {msg.citations && msg.citations.length > 0 && (
                          <div className="mt-4 pt-3 border-t border-zinc-800/50">
                            <p className="text-zinc-500 text-xs font-bold uppercase tracking-wider mb-2 flex items-center">
                                <FileText size={12} className="mr-1"/> References
                            </p>
                            <div className="space-y-2">
                                {msg.citations.map((cite: any, idx: number) => (
                                <div key={idx} className="bg-black/30 p-2 rounded border border-zinc-800 text-xs text-zinc-400 font-mono">
                                    <span className="text-zinc-500">Page {cite.page}:</span> "...{cite.text ? cite.text.substring(0, 80) : ''}..."
                                </div>
                                ))}
                            </div>
                          </div>
                        )}
                      </div>
                    </div>
                  ))}
                  {loading && (
                    <div className="flex justify-start animate-pulse">
                        <div className="bg-zinc-900 rounded-2xl p-4 flex items-center text-zinc-400 text-sm">
                            <Loader2 className="animate-spin mr-2 h-4 w-4"/> AI is thinking...
                        </div>
                    </div>
                  )}
                </div>
              )}
            </div>

            {/* Input Area */}
            <div className="p-4 bg-black border-t border-zinc-900">
                <div className="max-w-3xl mx-auto relative">
                    <input 
                        type="text" 
                        value={query}
                        onChange={(e) => setQuery(e.target.value)}
                        onKeyDown={(e) => e.key === 'Enter' && handleSearch()}
                        placeholder="Message Agentic AI..."
                        className="w-full bg-zinc-900 border border-zinc-800 text-white rounded-full px-6 py-4 pr-12 focus:outline-none focus:ring-2 focus:ring-zinc-700 focus:border-transparent placeholder:text-zinc-600 shadow-lg"
                    />
                    <button 
                        onClick={handleSearch}
                        disabled={!query}
                        className="absolute right-2 top-2 p-2 bg-white text-black rounded-full hover:bg-zinc-200 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                    >
                        <Send size={18} />
                    </button>
                </div>
                <div className="text-center text-[10px] text-zinc-600 mt-2">
                    AI can make mistakes. Please verify important information.
                </div>
            </div>
          </>
        )}

        {/* UPLOAD VIEW */}
        {activeTab === 'upload' && (
          <div className="flex-1 flex items-center justify-center p-8">
            <div className="max-w-md w-full bg-zinc-900 p-8 rounded-3xl border border-zinc-800 shadow-2xl text-center">
                <div className="w-20 h-20 bg-zinc-800 rounded-full flex items-center justify-center mx-auto mb-6">
                    <Upload size={32} className="text-white"/>
                </div>
                <h2 className="text-2xl font-bold text-white mb-2">Upload Knowledge</h2>
                <p className="text-zinc-400 text-sm mb-8">Supported formats: PDF, DOCX, CSV, JPG, PNG, MP3, WAV</p>
                
                <input type="file" ref={fileInputRef} onChange={handleUpload} className="hidden" />
                <button 
                    onClick={() => fileInputRef.current?.click()}
                    className="w-full bg-white text-black font-bold py-3.5 rounded-xl hover:bg-zinc-200 transition-all active:scale-95"
                >
                    Choose File
                </button>
                
                {uploadStatus && (
                    <div className="mt-6 p-3 bg-zinc-950 rounded-lg border border-zinc-800 text-xs text-green-400 font-mono">
                        {uploadStatus}
                    </div>
                )}
            </div>
          </div>
        )}

        {/* APPROVALS VIEW */}
        {activeTab === 'approvals' && (
          <div className="flex-1 p-8 md:p-12 overflow-y-auto">
            <div className="max-w-4xl mx-auto">
                <h2 className="text-3xl font-bold text-white mb-8 flex items-center">
                    <ShieldAlert className="mr-3 text-red-500" size={32}/> Action Center
                </h2>
                
                <div className="space-y-4">
                {pendingActions.length === 0 ? (
                    <div className="text-center py-20 border-2 border-dashed border-zinc-900 rounded-2xl text-zinc-600">
                        <p>No actions pending approval.</p>
                    </div>
                ) : ( 
                pendingActions.map((action) => (
                    <div key={action.id} className="bg-zinc-900 border border-zinc-800 p-6 rounded-2xl flex items-center justify-between shadow-lg group hover:border-zinc-700 transition-all">
                        <div>
                            <div className="flex items-center gap-3 mb-2">
                                <span className="bg-blue-500/10 text-blue-400 text-[10px] px-2 py-1 rounded-full uppercase font-bold tracking-wider border border-blue-500/20">
                                    {action.action_type || action.type}
                                </span>
                                <span className="text-zinc-500 text-xs">
                                    Confidence: {((action.payload?.confidence || action.confidence) * 100).toFixed(0)}%
                                </span>
                            </div>
                            <p className="text-lg font-medium text-zinc-200">
                                {action.payload?.description || action.description}
                            </p>
                        </div>
                        <div className="flex gap-3">
                            <button onClick={() => handleApprove(action.id, false)} className="p-3 hover:bg-red-500/10 text-red-500 rounded-xl transition border border-transparent hover:border-red-500/30">
                                <X size={24}/>
                            </button>
                            <button onClick={() => handleApprove(action.id, true)} className="p-3 bg-white hover:bg-zinc-200 text-black rounded-xl transition shadow-lg shadow-white/10">
                                <Check size={24}/>
                            </button>
                        </div>
                    </div>
                )))}
                </div>
            </div>
          </div>
        )}

      </div>
    </main>
  );
}