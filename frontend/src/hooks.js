import { useState, useEffect } from 'react';

const API_Base = "http://localhost:8001";
export { API_Base };

export const useLabels = () => {
    const [labels, setLabels] = useState([]);
    useEffect(() => {
        fetch(`${API_Base}/labels`)
            .then(res => res.json())
            .then(data => setLabels(data))
            .catch(err => console.error("Failed to fetch labels", err));
    }, []);
    return labels;
};

export const useEmails = (label, query, page, startDate, endDate) => {
    const [emails, setEmails] = useState([]);
    const [total, setTotal] = useState(0);
    const [loading, setLoading] = useState(false);
    const [hasMore, setHasMore] = useState(true);

    useEffect(() => {
        // Reset emails when filters change
        setEmails([]);
        setHasMore(true);
    }, [label, query, startDate, endDate]);

    useEffect(() => {
        setLoading(true);
        let url = `${API_Base}/search?page=${page}&size=20`;
        if (label) url += `&label=${encodeURIComponent(label)}`;
        if (query) url += `&q=${encodeURIComponent(query)}`;
        if (startDate) url += `&start_date=${encodeURIComponent(startDate)}`;
        if (endDate) url += `&end_date=${encodeURIComponent(endDate)}`;

        fetch(url)
            .then(res => res.json())
            .then(data => {
                const items = data.items || [];
                if (page === 1) {
                    setEmails(items);
                } else {
                    setEmails(prev => [...prev, ...items]);
                }
                setTotal(data.total || 0);
                setHasMore(items.length === 20); // If we got a full page, there might be more
                setLoading(false);
            })
            .catch(err => {
                console.error("Failed to fetch emails", err);
                setLoading(false);
            });
    }, [label, query, page, startDate, endDate]);

    return { emails, total, loading, hasMore };
};

export const useEmailDetail = (id) => {
    const [email, setEmail] = useState(null);
    const [loading, setLoading] = useState(false);

    useEffect(() => {
        if (!id) return;
        setLoading(true);
        fetch(`${API_Base}/email/${id}`)
            .then(res => res.json())
            .then(data => {
                setEmail(data);
                setLoading(false);
            })
            .catch(err => {
                console.error(err);
                setLoading(false);
            });
    }, [id]);

    return { email, loading };
}
