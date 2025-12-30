import { useLabels } from '../hooks';

export default function Sidebar({ selectedLabel, onSelectLabel }) {
    const labels = useLabels();

    // Group labels if they have slashes? For now simple list.
    // Prioritize standard labels
    const standard = ["Inbox", "Sent", "Trash", "Spam", "Drafts", "Important", "Starred"];
    const safeLabels = Array.isArray(labels) ? labels : [];
    const others = safeLabels.filter(l => !standard.includes(l));
    const sysLabels = standard.filter(l => safeLabels.includes(l));

    return (
        <div className="w-64 bg-white border-r h-full flex flex-col overflow-y-auto">
            <div className="p-4 text-xl font-bold text-red-600">Gmail Archive</div>
            <nav className="flex-1">
                {sysLabels.map(label => (
                    <button
                        key={label}
                        onClick={() => onSelectLabel(label)}
                        className={`w-full text-left px-6 py-2 hover:bg-gray-100 rounded-r-full mr-2 ${selectedLabel === label ? 'bg-red-50 text-red-600 font-semibold' : 'text-gray-700'}`}
                    >
                        {label}
                    </button>
                ))}

                {others.length > 0 && <div className="mt-4 px-6 text-xs font-semibold text-gray-500 uppercase">Labels</div>}

                {others.map(label => (
                    <button
                        key={label}
                        onClick={() => onSelectLabel(label)}
                        className={`w-full text-left px-6 py-1.5 hover:bg-gray-100 rounded-r-full mr-2 truncate text-sm ${selectedLabel === label ? 'bg-red-50 text-red-600 font-semibold' : 'text-gray-600'}`}
                        title={label}
                    >
                        {label}
                    </button>
                ))}
            </nav>
        </div>
    );
}
