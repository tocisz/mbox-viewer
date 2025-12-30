import { useEmails } from '../hooks';

export default function EmailList({ label, query, onSelectEmail, selectedEmailId }) {
    const { emails, loading } = useEmails(label, query, 1);

    if (loading) return <div className="p-4">Loading...</div>;

    return (
        <div className="flex-1 bg-white flex flex-col overflow-y-auto">
            {emails.length === 0 && <div className="p-8 text-center text-gray-500">No emails found</div>}
            {emails.map(email => (
                <div
                    key={email.id}
                    onClick={() => onSelectEmail(email.id)}
                    className={`border-b px-4 py-3 cursor-pointer hover:shadow-md transition-shadow flex items-center gap-4 ${selectedEmailId === email.id ? 'bg-blue-50 border-l-4 border-l-blue-500' : 'hover:bg-gray-50'}`}
                >
                    <div className="w-48 font-semibold truncate text-gray-900">{email.sender}</div>
                    <div className="flex-1 min-w-0 flex items-center gap-2">
                        <span className="font-medium text-gray-800 truncate">{email.subject}</span>
                        {email.has_attachment && <span title="Has attachment">📎</span>}
                        <span className="text-gray-500 mx-1">-</span>
                        <span className="text-gray-500 truncate">{email.snippet}</span>
                    </div>
                    <div className="text-xs text-gray-500 font-medium whitespace-nowrap w-24 text-right">
                        {new Date(email.date).toLocaleDateString()}
                    </div>
                </div>
            ))}
        </div>
    );
}
