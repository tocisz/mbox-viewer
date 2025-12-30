import { useEmailDetail } from '../hooks';

export default function EmailDetail({ id }) {
    const { email, loading } = useEmailDetail(id);

    if (loading) return <div className="p-8 text-center">Loading message...</div>;
    if (!email) return <div className="p-8 text-center text-gray-500">Select an email to view</div>;

    return (
        <div className="h-full flex flex-col bg-white p-6 overflow-y-auto">
            <h1 className="text-2xl font-normal text-gray-900 mb-4">{email.subject}</h1>
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

            <div className="border-t pt-6"
                dangerouslySetInnerHTML={{ __html: email.body_html }}
            />
        </div>
    );
}
