import { useEmailDetail } from '../hooks';

export default function EmailDetail({ id, onClose, onSelectLabel }) {
    const { email, loading } = useEmailDetail(id);

    if (loading) return <div className="p-8 text-center">Loading message...</div>;
    if (!email) return <div className="p-8 text-center text-gray-500">Select an email to view</div>;

    return (
        <div className="h-full flex flex-col bg-white p-6 overflow-y-auto relative">
            <button
                onClick={onClose}
                className="absolute top-4 right-4 text-gray-400 hover:text-gray-600 transition-colors p-2 text-2xl"
                aria-label="Close"
            >
                ✕
            </button>
            <h1 className="text-2xl font-normal text-gray-900 mb-4 pr-10">{email.subject}</h1>
            <div className="flex items-start justify-between mb-6">
                <div className="flex gap-3">
                    <div className="w-10 h-10 rounded-full bg-blue-600 flex items-center justify-center text-white font-bold text-lg">
                        {email.sender.charAt(0).toUpperCase()}
                    </div>
                    <div>
                        <div className="font-bold text-gray-900">{email.sender}</div>
                        <div className="text-sm text-gray-500">to {email.to || "me"}</div>
                    </div>
                </div>
                <div className="text-sm text-gray-500">
                    {new Date(email.date).toLocaleString()}
                </div>
            </div>

            {email.labels && email.labels.length > 0 && (
                <div className="flex flex-wrap gap-2 mb-6">
                    {email.labels.map(label => (
                        <button
                            key={label}
                            onClick={() => onSelectLabel(label)}
                            className="px-2 py-0.5 bg-gray-100 text-gray-600 rounded text-xs font-medium border hover:bg-gray-200 hover:text-gray-800 transition-colors"
                        >
                            {label}
                        </button>
                    ))}
                </div>
            )}

            <div className="border-t pt-6 mb-8"
                dangerouslySetInnerHTML={{ __html: email.body_html }}
            />

            {email.attachments && email.attachments.length > 0 && (
                <div className="border-t pt-6">
                    <h3 className="text-lg font-medium text-gray-900 mb-4 flex items-center gap-2">
                        <span>📎</span> Attachments ({email.attachments.length})
                    </h3>
                    <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
                        {email.attachments.map((att, index) => (
                            <div key={index} className="border rounded-lg p-3 flex flex-col gap-2 hover:bg-gray-50 transition-colors">
                                <div className="font-medium text-gray-800 truncate" title={att.filename}>
                                    {att.filename}
                                </div>
                                <div className="text-xs text-gray-500">
                                    {(att.size / 1024).toFixed(1)} KB
                                </div>
                                <a
                                    href={`http://localhost:8000/attachment/${att.path}`}
                                    className="mt-2 text-blue-600 hover:text-blue-800 text-sm font-medium flex items-center gap-1"
                                    download={att.filename}
                                    target="_blank"
                                    rel="noopener noreferrer"
                                >
                                    <span>⬇️</span> Download
                                </a>
                            </div>
                        ))}
                    </div>
                </div>
            )}
        </div>
    );
}
