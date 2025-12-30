import { useState, useEffect } from 'react';

const API_Base = "http://localhost:8000";

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

export const useEmails = (label, query, page) => {
    const [emails, setEmails] = useState([]);
    const [total, setTotal] = useState(0);
    const [loading, setLoading] = useState(false);

    useEffect(() => {
        setLoading(true);
        let url = `${API_Base}/search?page=${page}&size=20`;
        if (label) url += `&label=${encodeURIComponent(label)}`;
        if (query) url += `&q=${encodeURIComponent(query)}`;

        fetch(url)
            .then(res => res.json())
            .then(data => {
                if (data && data.items) {
                    setEmails(data.items);
                    setTotal(data.total);
                } else {
                    setEmails([]);
                    setTotal(0);
                }
                setLoading(false);
            })
            .catch(err => {
                console.error("Failed to fetch emails", err);
                setLoading(false);
            });
    }, [label, query, page]);

    return { emails, total, loading };
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
