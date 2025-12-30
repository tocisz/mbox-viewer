import { useState } from 'react';
import Sidebar from './components/Sidebar';
import EmailList from './components/EmailList';
import EmailDetail from './components/EmailDetail';

export default function App() {
  const [selectedLabel, setSelectedLabel] = useState("Inbox");
  const [searchQuery, setSearchQuery] = useState("");
  const [searchInput, setSearchInput] = useState("");
  const [selectedEmailId, setSelectedEmailId] = useState(null);
  const [startDate, setStartDate] = useState("");
  const [endDate, setEndDate] = useState("");

  const handleSearch = (e) => {
    e.preventDefault();
    setSearchQuery(searchInput);
    if (searchInput) setSelectedLabel(null); // Clear label on search
  };

  return (
    <div className="flex h-screen w-screen flex-col bg-gray-100 text-sm">
      {/* Header */}
      <header className="bg-white border-b px-4 py-2 flex items-center justify-between shadow-sm z-10">
        <div className="flex items-center gap-4 w-full">
          <div className="w-64 font-bold text-xl text-gray-700 flex items-center gap-2">
            <span>✉️</span> ArchiveViewer
          </div>
          <form onSubmit={handleSearch} className="flex-1 max-w-xl">
            <div className="relative">
              <input
                type="text"
                className="w-full bg-gray-100 border-none rounded-lg py-2.5 px-4 focus:bg-white focus:shadow shadow-inner transition-all outline-none"
                placeholder="Search mail"
                value={searchInput}
                onChange={e => setSearchInput(e.target.value)}
              />
            </div>
          </form>
          <div className="flex items-center gap-2 text-gray-600">
            <div className="flex items-center gap-1">
              <label className="text-xs font-medium">From:</label>
              <input
                type="date"
                className="bg-gray-100 border-none rounded-md py-1 px-2 outline-none focus:bg-white focus:ring-1 focus:ring-blue-400"
                value={startDate}
                onChange={e => setStartDate(e.target.value)}
              />
            </div>
            <div className="flex items-center gap-1">
              <label className="text-xs font-medium">To:</label>
              <input
                type="date"
                className="bg-gray-100 border-none rounded-md py-1 px-2 outline-none focus:bg-white focus:ring-1 focus:ring-blue-400"
                value={endDate}
                onChange={e => setEndDate(e.target.value)}
              />
            </div>
            {(startDate || endDate) && (
              <button
                onClick={() => { setStartDate(""); setEndDate(""); }}
                className="text-xs text-blue-600 hover:text-blue-800 font-medium"
              >
                Clear
              </button>
            )}
          </div>
        </div>
      </header>

      {/* Main Layout */}
      <div className="flex flex-1 overflow-hidden">
        <Sidebar selectedLabel={selectedLabel} onSelectLabel={(l) => { setSelectedLabel(l); setSearchQuery(""); setSelectedEmailId(null); }} />

        {/* Split View */}
        <div className="flex-1 flex overflow-hidden">

          {/* List View */}
          <div className={`${selectedEmailId ? 'w-1/3 border-r hidden md:flex' : 'w-full'} flex flex-col`}>
            <EmailList
              label={selectedLabel}
              query={searchQuery}
              startDate={startDate}
              endDate={endDate}
              onSelectEmail={setSelectedEmailId}
              selectedEmailId={selectedEmailId}
            />
          </div>

          {/* Detail View */}
          {selectedEmailId && (
            <div className="flex-1 flex flex-col overflow-hidden bg-white relative">
              <EmailDetail
                id={selectedEmailId}
                onClose={() => setSelectedEmailId(null)}
                onSelectLabel={(label) => {
                  setSelectedLabel(label);
                  setSearchQuery("");
                  setSelectedEmailId(null);
                }}
              />
            </div>
          )}

          {/* Empty State for Detail View (Desktop) */}
          {!selectedEmailId && <div className="hidden md:flex flex-1 items-center justify-center text-gray-400 bg-gray-50 flex-col gap-4">
            <div className="text-6xl">📬</div>
            <div>Select an email to read</div>
          </div>}
        </div>
      </div>
    </div>
  );
}
